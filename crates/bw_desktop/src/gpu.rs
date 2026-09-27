//! Native GPU transport for Brinewake's physical client-sized RGBA frame.
//!
//! The software compositor owns the frame bytes. This module only uploads that
//! frame and presents it through a wgpu surface. It deliberately has no scene,
//! simulation, or asset knowledge.

use std::sync::{Arc, Mutex};

use winit::window::Window;

const BYTES_PER_PIXEL: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SurfaceFormatPair {
    /// A linear/unorm upload paired with a linear/unorm target keeps source
    /// palette bytes as source palette values on the common desktop path.
    Linear {
        surface: wgpu::TextureFormat,
        upload: wgpu::TextureFormat,
    },
    /// An sRGB upload is decoded when sampled and encoded by the sRGB target,
    /// preserving the color-space contract when only an sRGB target exists.
    Srgb {
        surface: wgpu::TextureFormat,
        upload: wgpu::TextureFormat,
    },
}

impl SurfaceFormatPair {
    fn surface(self) -> wgpu::TextureFormat {
        match self {
            Self::Linear { surface, .. } | Self::Srgb { surface, .. } => surface,
        }
    }

    fn upload(self) -> wgpu::TextureFormat {
        match self {
            Self::Linear { upload, .. } | Self::Srgb { upload, .. } => upload,
        }
    }
}

/// Owns the wgpu resources used to present the software-composed frame.
///
/// `Gpu` is intended to live on the native application thread. It retains the
/// `Arc<Window>` because wgpu's safe surface constructor may retain the window
/// handle for the lifetime of the surface.
pub struct Gpu {
    // Keep the surface before the window so the surface is dropped first. The
    // surface may retain an owned window handle internally.
    surface: wgpu::Surface<'static>,
    window: Arc<Window>,
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    surface_format: wgpu::TextureFormat,
    upload_format: wgpu::TextureFormat,
    frame_resources: Option<FrameResources>,
    width: u32,
    height: u32,
    frame_size: Option<(u32, u32)>,
    device_error: Arc<Mutex<Option<String>>>,
    resize_error: Option<String>,
    adapter_info: wgpu::AdapterInfo,
}

impl Gpu {
    /// Creates a native GPU presenter for `window`.
    ///
    /// Initialization is synchronous at this boundary because the desktop app
    /// needs a ready presenter before it enters its redraw loop. `pollster`
    /// drives the wgpu adapter/device futures without adding an async runtime.
    pub fn new(window: Arc<Window>) -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|error| format!("failed to create wgpu surface: {error:?}"))?;

        let adapter_options = wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            ..Default::default()
        };
        let adapter = pollster::block_on(instance.request_adapter(&adapter_options))
            .map_err(|error| format!("failed to find a presentable GPU adapter: {error:?}"))?;
        let adapter_info = adapter.get_info();

        let initial_size = window.inner_size();
        let pair = choose_surface_format(&surface.get_capabilities(&adapter).formats)
            .ok_or_else(|| "surface has no supported 8-bit RGBA/BGRA format".to_owned())?;
        let max_texture_dimension_2d = adapter.limits().max_texture_dimension_2d;
        validate_frame_dimensions(
            initial_size.width,
            initial_size.height,
            max_texture_dimension_2d,
        )?;
        let mut config = make_surface_config(
            &surface,
            &adapter,
            pair.surface(),
            initial_size.width,
            initial_size.height,
        )?;

        let device_error = Arc::new(Mutex::new(None));
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .map_err(|error| format!("failed to create wgpu device: {error:?}"))?;
        install_error_handlers(&device, &device_error);
        validate_frame_dimensions(
            initial_size.width,
            initial_size.height,
            device.limits().max_texture_dimension_2d,
        )?;

        let frame_resources = (initial_size.width != 0 && initial_size.height != 0).then(|| {
            create_frame_resources(
                &device,
                pair.surface(),
                pair.upload(),
                initial_size.width,
                initial_size.height,
            )
        });
        if let Some(error) = take_device_error(&device_error) {
            return Err(error);
        }

        // A zero-sized surface cannot be configured. Keep the config values
        // available for the first non-zero resize, but skip configuration now.
        if initial_size.width != 0 && initial_size.height != 0 {
            config.width = initial_size.width;
            config.height = initial_size.height;
            surface.configure(&device, &config);
        }

        if let Some(error) = take_device_error(&device_error) {
            return Err(error);
        }

        let gpu = Self {
            surface,
            window,
            instance,
            adapter,
            device,
            queue,
            config,
            surface_format: pair.surface(),
            upload_format: pair.upload(),
            frame_resources,
            width: initial_size.width,
            height: initial_size.height,
            frame_size: (initial_size.width != 0 && initial_size.height != 0)
                .then_some((initial_size.width, initial_size.height)),
            device_error,
            resize_error: None,
            adapter_info,
        };

        // `configure` above can enqueue an asynchronous validation error. Poll
        // once before handing the presenter to the application so initialization
        // cannot appear successful after a failed surface setup.
        gpu.poll_device()?;
        Ok(gpu)
    }

    /// Updates the physical client size and reconfigures the surface.
    ///
    /// The public contract intentionally has no return value because winit
    /// resize events are not fallible. Configuration failures are retained and
    /// returned by the next `present` call. Zero dimensions are safe: no
    /// surface is configured and no frame texture is allocated for them.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        if width == 0 || height == 0 {
            return;
        }

        match self.configure_for_size() {
            Ok(()) => self.resize_error = None,
            Err(error) => self.resize_error = Some(error),
        }
    }

    /// Uploads one exact physical-size RGBA frame and presents it fullscreen.
    ///
    /// The frame texture and surface have identical dimensions, so the
    /// fullscreen pass has one source texel for each physical client pixel.
    pub fn present(&mut self, rgba: &[u8]) -> Result<(), String> {
        // There is no valid physical target while minimized. Skip before
        // validating the stale caller buffer so an in-flight redraw cannot
        // turn a zero-size surface into a frame-length error.
        if self.width == 0 || self.height == 0 {
            return Ok(());
        }

        if let Some(error) = self.resize_error.take() {
            return Err(error);
        }

        let layout = frame_layout(self.width, self.height)?;
        if rgba.len() != layout.byte_len {
            return Err(format!(
                "invalid frame length: expected {} RGBA bytes for {}x{}, got {}",
                layout.byte_len,
                self.width,
                self.height,
                rgba.len()
            ));
        }

        self.poll_device()?;

        if self.frame_resources.is_none() {
            return Err(
                "GPU frame resources are unavailable for the current surface size".to_owned(),
            );
        }

        let mut surface_retries = 0;
        loop {
            match self.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame) => {
                    self.upload_frame(rgba)?;
                    self.draw_frame(frame)?;
                    return Ok(());
                }
                wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                    self.upload_frame(rgba)?;
                    self.draw_frame(frame)?;
                    // A suboptimal frame is still valid. Reconfigure after
                    // presenting it, when no SurfaceTexture is alive.
                    self.configure_for_size()?;
                    return Ok(());
                }
                wgpu::CurrentSurfaceTexture::Outdated if surface_retries == 0 => {
                    surface_retries += 1;
                    self.configure_for_size()?;
                }
                wgpu::CurrentSurfaceTexture::Lost if surface_retries == 0 => {
                    surface_retries += 1;
                    self.recreate_surface()?;
                }
                wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                    // These are transient acquisition states. The next redraw
                    // will retry once the compositor/device is available.
                    return Ok(());
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    return Err(self
                        .take_device_error()
                        .unwrap_or_else(|| "wgpu surface validation failure".to_owned()));
                }
                wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                    return Err("wgpu surface could not be recovered after one retry".to_owned());
                }
            }
        }
    }

    /// Returns adapter, surface, and physical-frame diagnostics for logs and
    /// the platform capability report.
    pub fn info(&self) -> String {
        let frame = self
            .frame_size
            .map(|(width, height)| format!("{width}x{height}"))
            .unwrap_or_else(|| "unavailable".to_owned());
        let device_error = self
            .device_error
            .lock()
            .ok()
            .and_then(|error| error.clone());
        format!(
            "adapter={} backend={:?} device_type={:?} driver={} driver_info={} surface={}x{} format={:?} upload_format={:?} frame={}{}",
            self.adapter_info.name,
            self.adapter_info.backend,
            self.adapter_info.device_type,
            self.adapter_info.driver,
            self.adapter_info.driver_info,
            self.width,
            self.height,
            self.surface_format,
            self.upload_format,
            frame,
            device_error
                .map(|error| format!(" device_error={error}"))
                .unwrap_or_default(),
        )
    }

    fn draw_frame(&mut self, frame: wgpu::SurfaceTexture) -> Result<(), String> {
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let resources = self
            .frame_resources
            .as_ref()
            .ok_or_else(|| "GPU frame resources are unavailable".to_owned())?;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("brinewake-present-encoder"),
            });
        {
            let color_attachment = wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("brinewake-present-pass"),
                color_attachments: &[Some(color_attachment)],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&resources.pipeline);
            pass.set_bind_group(0, &resources.bind_group, &[]);
            pass.set_viewport(0.0, 0.0, self.width as f32, self.height as f32, 0.0, 1.0);
            pass.draw(0..6, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        self.poll_device()?;
        self.window.pre_present_notify();
        self.queue.present(frame);
        self.poll_device()
    }

    fn upload_frame(&self, rgba: &[u8]) -> Result<(), String> {
        let layout = frame_layout(self.width, self.height)?;
        if rgba.len() != layout.byte_len {
            return Err(format!(
                "invalid frame length: expected {} RGBA bytes for {}x{}, got {}",
                layout.byte_len,
                self.width,
                self.height,
                rgba.len()
            ));
        }
        let resources = self
            .frame_resources
            .as_ref()
            .ok_or_else(|| "GPU frame resources are unavailable".to_owned())?;
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &resources.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(layout.bytes_per_row),
                rows_per_image: Some(layout.height),
            },
            wgpu::Extent3d {
                width: layout.width,
                height: layout.height,
                depth_or_array_layers: 1,
            },
        );
        self.poll_device()
    }

    fn configure_for_size(&mut self) -> Result<(), String> {
        if self.width == 0 || self.height == 0 {
            return Ok(());
        }

        let capabilities = self.surface.get_capabilities(&self.adapter);
        let pair = choose_surface_format(&capabilities.formats)
            .ok_or_else(|| "surface has no supported 8-bit RGBA/BGRA format".to_owned())?;
        validate_frame_dimensions(
            self.width,
            self.height,
            self.device.limits().max_texture_dimension_2d,
        )?;

        let size_changed = self.frame_size != Some((self.width, self.height));
        let format_changed =
            pair.surface() != self.surface_format || pair.upload() != self.upload_format;
        if size_changed || format_changed || self.frame_resources.is_none() {
            let resources = create_frame_resources(
                &self.device,
                pair.surface(),
                pair.upload(),
                self.width,
                self.height,
            );
            if let Some(error) = self.take_device_error() {
                return Err(error);
            }
            self.surface_format = pair.surface();
            self.upload_format = pair.upload();
            self.frame_resources = Some(resources);
            self.frame_size = Some((self.width, self.height));
        }

        let mut config = make_surface_config(
            &self.surface,
            &self.adapter,
            self.surface_format,
            self.width,
            self.height,
        )?;
        config.present_mode = capabilities
            .present_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::PresentMode::Fifo)
            .unwrap_or(config.present_mode);
        config.desired_maximum_frame_latency = 2;
        self.surface.configure(&self.device, &config);
        self.config = config;
        self.poll_device()
    }

    fn recreate_surface(&mut self) -> Result<(), String> {
        let surface = self
            .instance
            .create_surface(self.window.clone())
            .map_err(|error| format!("failed to recreate lost wgpu surface: {error:?}"))?;
        self.surface = surface;
        self.configure_for_size()
    }

    fn poll_device(&self) -> Result<(), String> {
        self.device
            .poll(wgpu::PollType::Poll)
            .map_err(|error| format!("wgpu device poll failed: {error:?}"))?;
        self.take_device_error().map_or(Ok(()), Err)
    }

    fn take_device_error(&self) -> Option<String> {
        take_device_error(&self.device_error)
    }
}

struct FrameResources {
    texture: wgpu::Texture,
    // Keep the view and sampler handles alive alongside the bind group.
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
}

fn create_frame_resources(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
    upload_format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> FrameResources {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("brinewake-physical-frame"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: upload_format,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("brinewake-nearest-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("brinewake-frame-bind-group-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("brinewake-frame-bind-group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("brinewake-present-shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("gpu.wgsl").into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("brinewake-present-pipeline-layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("brinewake-present-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });

    FrameResources {
        texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        pipeline,
    }
}

fn make_surface_config(
    surface: &wgpu::Surface<'_>,
    adapter: &wgpu::Adapter,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> Result<wgpu::SurfaceConfiguration, String> {
    let mut config = surface
        .get_default_config(adapter, width, height)
        .ok_or_else(|| "GPU adapter cannot present to the Brinewake surface".to_owned())?;
    config.format = format;
    Ok(config)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FrameLayout {
    width: u32,
    height: u32,
    bytes_per_row: u32,
    byte_len: usize,
}

fn frame_layout(width: u32, height: u32) -> Result<FrameLayout, String> {
    let bytes_per_row = width
        .checked_mul(BYTES_PER_PIXEL)
        .ok_or_else(|| format!("frame row is too wide for an RGBA upload: {width} pixels"))?;
    let byte_len_u64 = u64::from(bytes_per_row)
        .checked_mul(u64::from(height))
        .ok_or_else(|| format!("frame byte length overflows for {width}x{height}"))?;
    let byte_len = usize::try_from(byte_len_u64)
        .map_err(|_| format!("frame byte length does not fit usize for {width}x{height}"))?;
    Ok(FrameLayout {
        width,
        height,
        bytes_per_row,
        byte_len,
    })
}

fn validate_frame_dimensions(
    width: u32,
    height: u32,
    max_texture_dimension_2d: u32,
) -> Result<(), String> {
    if width == 0 || height == 0 {
        return Ok(());
    }
    if width > max_texture_dimension_2d || height > max_texture_dimension_2d {
        return Err(format!(
            "surface size {width}x{height} exceeds GPU texture limit {max_texture_dimension_2d}"
        ));
    }
    frame_layout(width, height).map(|_| ())
}

fn choose_surface_format(formats: &[wgpu::TextureFormat]) -> Option<SurfaceFormatPair> {
    // Prefer non-sRGB targets so an 8-bit software palette is not implicitly
    // transformed. If the platform exposes only sRGB, pair it with an sRGB
    // upload texture so decode/encode happen symmetrically.
    for format in [
        wgpu::TextureFormat::Bgra8Unorm,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    ] {
        if formats.contains(&format) {
            return Some(
                if matches!(
                    format,
                    wgpu::TextureFormat::Bgra8UnormSrgb | wgpu::TextureFormat::Rgba8UnormSrgb
                ) {
                    SurfaceFormatPair::Srgb {
                        surface: format,
                        upload: wgpu::TextureFormat::Rgba8UnormSrgb,
                    }
                } else {
                    SurfaceFormatPair::Linear {
                        surface: format,
                        upload: wgpu::TextureFormat::Rgba8Unorm,
                    }
                },
            );
        }
    }
    None
}

fn install_error_handlers(device: &wgpu::Device, device_error: &Arc<Mutex<Option<String>>>) {
    let uncaptured_error = Arc::clone(device_error);
    device.on_uncaptured_error(Arc::new(move |error| {
        record_device_error(
            &uncaptured_error,
            format!("wgpu uncaptured error: {error:?}"),
        );
    }));

    let lost_error = Arc::clone(device_error);
    device.set_device_lost_callback(move |reason, message| {
        record_device_error(
            &lost_error,
            format!("wgpu device lost ({reason:?}): {message}"),
        );
    });
}

fn record_device_error(slot: &Arc<Mutex<Option<String>>>, error: String) {
    if let Ok(mut recorded) = slot.lock()
        && recorded.is_none()
    {
        *recorded = Some(error);
    }
}

fn take_device_error(slot: &Arc<Mutex<Option<String>>>) -> Option<String> {
    slot.lock().ok().and_then(|mut recorded| recorded.take())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_layout_matches_physical_upload_rows() {
        let layout = frame_layout(1366, 768).expect("layout should fit");
        assert_eq!(layout.width, 1366);
        assert_eq!(layout.height, 768);
        assert_eq!(layout.bytes_per_row, 1366 * 4);
        assert_eq!(layout.byte_len, 1366usize * 768 * 4);
    }

    #[test]
    fn frame_layout_rejects_row_overflow() {
        let error = frame_layout(u32::MAX, 1).expect_err("row must not overflow");
        assert!(error.contains("row is too wide"));
    }

    #[test]
    fn zero_size_frame_is_valid_for_minimized_surface() {
        let layout = frame_layout(0, 0).expect("zero-size skip needs a valid empty layout");
        assert_eq!(layout.byte_len, 0);
        assert_eq!(layout.bytes_per_row, 0);
        assert!(validate_frame_dimensions(0, 0, 1).is_ok());
    }

    #[test]
    fn rejects_dimensions_above_adapter_texture_limit() {
        let error = validate_frame_dimensions(2049, 360, 2048)
            .expect_err("oversized surface must fail before texture creation");
        assert!(error.contains("exceeds GPU texture limit 2048"));
    }

    #[test]
    fn prefers_linear_surface_formats() {
        let pair = choose_surface_format(&[
            wgpu::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Bgra8Unorm,
        ])
        .expect("format should be selected");
        assert_eq!(pair.surface(), wgpu::TextureFormat::Bgra8Unorm);
        assert_eq!(pair.upload(), wgpu::TextureFormat::Rgba8Unorm);
    }

    #[test]
    fn pairs_srgb_target_with_srgb_upload() {
        let pair = choose_surface_format(&[wgpu::TextureFormat::Rgba8UnormSrgb])
            .expect("format should be selected");
        assert_eq!(pair.surface(), wgpu::TextureFormat::Rgba8UnormSrgb);
        assert_eq!(pair.upload(), wgpu::TextureFormat::Rgba8UnormSrgb);
    }

    #[test]
    fn pairs_bgra_srgb_target_with_rgba_srgb_upload() {
        let pair = choose_surface_format(&[wgpu::TextureFormat::Bgra8UnormSrgb])
            .expect("format should be selected");
        assert_eq!(pair.surface(), wgpu::TextureFormat::Bgra8UnormSrgb);
        assert_eq!(pair.upload(), wgpu::TextureFormat::Rgba8UnormSrgb);
    }

    #[test]
    fn rejects_unsupported_surface_formats() {
        assert!(choose_surface_format(&[wgpu::TextureFormat::R16Float]).is_none());
    }
}
