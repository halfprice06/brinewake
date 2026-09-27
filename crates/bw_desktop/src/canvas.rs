//! Small native-resolution rasterizer. All game artwork is composed on this pixel grid.
use bw_core::{CANVAS_H, CANVAS_W};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

const MAX_CANVAS_WIDTH: u32 = 8192;
const MAX_CANVAS_HEIGHT: u32 = 8192;

pub type Color = [u8; 4];
pub const INK: Color = [19, 31, 39, 255];
pub const PANEL: Color = [28, 45, 54, 255];
pub const EDGE: Color = [71, 96, 102, 255];
pub const WHITE: Color = [239, 228, 197, 255];
pub const MUTED: Color = [151, 169, 167, 255];
pub const GOLD: Color = [236, 177, 96, 255];
pub const JADE: Color = [127, 194, 164, 255];
pub const RED: Color = [226, 108, 84, 255];
/// The Saltglass Compact's accent: its cobalt glaze.
pub const COBALT: Color = [120, 150, 236, 255];
/// The second opponent's colour in a match of three.
pub const VIOLET: Color = [178, 138, 236, 255];

/// The authored dock-console face has seven lit columns and one pixel of
/// tracking.  Keep this separate from the original six-pixel 5×7 advance so
/// existing menus, buttons, and captures retain their geometry.
pub const READABLE_ADVANCE: i32 = 8;

/// Return the native-pixel width of a string drawn with [`Canvas::text_readable`].
pub fn text_readable_width(s: &str) -> i32 {
    s.chars().count() as i32 * READABLE_ADVANCE
}

#[derive(Clone)]
pub struct Canvas {
    pub pixels: Vec<u8>,
    width: u32,
    height: u32,
}
impl Default for Canvas {
    fn default() -> Self {
        Self::new(CANVAS_W, CANVAS_H)
    }
}
impl Canvas {
    /// Allocate a bounded native-resolution surface.
    ///
    /// The desktop renderer keeps the authored 640×360 surface as its
    /// default, while the scene renderer may request a larger temporary world
    /// surface for integer enlargement into the window frame.  A zero-sized or
    /// oversized surface is a programming error: allowing one through would
    /// make clipping and image export ambiguous.
    pub fn new(width: u32, height: u32) -> Self {
        Self::assert_dimensions(width, height);
        Self {
            pixels: vec![0; Self::byte_len(width, height)],
            width,
            height,
        }
    }

    /// Resize this surface and clear its contents.
    ///
    /// Clearing on resize prevents pixels from a prior scene size from
    /// surviving in newly exposed rows or columns.  Callers should still
    /// clear at the start of each frame when reusing a same-sized surface.
    #[allow(dead_code)]
    pub fn resize(&mut self, width: u32, height: u32) {
        Self::assert_dimensions(width, height);
        self.width = width;
        self.height = height;
        self.pixels.resize(Self::byte_len(width, height), 0);
        self.pixels.fill(0);
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    fn assert_dimensions(width: u32, height: u32) {
        assert!(
            width > 0 && height > 0 && width <= MAX_CANVAS_WIDTH && height <= MAX_CANVAS_HEIGHT,
            "Canvas dimensions must be within 1..={MAX_CANVAS_WIDTH} × 1..={MAX_CANVAS_HEIGHT}, got {width} × {height}"
        );
    }

    fn byte_len(width: u32, height: u32) -> usize {
        usize::try_from(u64::from(width) * u64::from(height) * 4)
            .expect("Canvas byte length must fit usize")
    }

    pub fn clear(&mut self, c: Color) {
        for p in self.pixels.chunks_exact_mut(4) {
            p.copy_from_slice(&c)
        }
    }
    pub fn pixel(&mut self, x: i32, y: i32, c: Color) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let i = ((y as u32 * self.width + x as u32) * 4) as usize;
        if c[3] == 255 {
            self.pixels[i..i + 4].copy_from_slice(&c)
        } else {
            let a = u32::from(c[3]);
            for (n, channel) in c.iter().take(3).enumerate() {
                self.pixels[i + n] = ((u32::from(*channel) * a
                    + u32::from(self.pixels[i + n]) * (255 - a))
                    / 255) as u8;
            }
            self.pixels[i + 3] = 255;
        }
    }
    /// Read one pixel back; `None` outside the canvas.
    pub fn get(&self, x: i32, y: i32) -> Option<Color> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        let i = ((y as u32 * self.width + x as u32) * 4) as usize;
        self.pixels[i..i + 4].try_into().ok()
    }
    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Color) {
        let x_end = x.saturating_add(w).min(self.width as i32);
        let y_end = y.saturating_add(h).min(self.height as i32);
        for yy in y.max(0)..y_end {
            for xx in x.max(0)..x_end {
                self.pixel(xx, yy, c)
            }
        }
    }
    pub fn line(&mut self, mut x: i32, mut y: i32, x1: i32, y1: i32, c: Color) {
        let dx = (x1 - x).abs();
        let sx = if x < x1 { 1 } else { -1 };
        let dy = -(y1 - y).abs();
        let sy = if y < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut remaining = 8192;
        while remaining > 0 {
            self.pixel(x, y, c);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = err * 2;
            if e2 >= dy {
                err += dy;
                x += sx
            }
            if e2 <= dx {
                err += dx;
                y += sy
            }
            remaining -= 1;
        }
    }
    pub fn frame(&mut self, x: i32, y: i32, w: i32, h: i32, c: Color) {
        self.rect(x, y, w, 1, c);
        self.rect(x, y + h - 1, w, 1, c);
        self.rect(x, y, 1, h, c);
        self.rect(x + w - 1, y, 1, h, c)
    }
    pub fn diamond(&mut self, x: i32, y: i32, rx: i32, ry: i32, c: Color) {
        if ry <= 0 {
            return;
        }
        for dy in -ry..=ry {
            let span = rx * (ry - dy.abs()) / ry;
            self.rect(x - span, y + dy, 2 * span + 1, 1, c)
        }
    }
    pub fn ellipse(&mut self, x: i32, y: i32, rx: i32, ry: i32, c: Color) {
        if rx <= 0 || ry <= 0 {
            return;
        }
        for dy in -ry..=ry {
            for dx in -rx..=rx {
                if dx * dx * ry * ry + dy * dy * rx * rx <= rx * rx * ry * ry {
                    self.pixel(x + dx, y + dy, c)
                }
            }
        }
    }
    pub fn text(&mut self, s: &str, x: i32, y: i32, c: Color) {
        self.text_scaled(s, x, y, c, 1)
    }
    pub fn text_scaled(&mut self, s: &str, x: i32, y: i32, c: Color, scale: i32) {
        for (i, ch) in s.to_ascii_uppercase().chars().enumerate() {
            let bits = glyph(ch);
            for (row, mask) in bits.iter().enumerate() {
                for col in 0..5 {
                    if mask & (1 << (4 - col)) != 0 {
                        self.rect(
                            x + i as i32 * 6 * scale + col * scale,
                            y + row as i32 * scale,
                            scale,
                            scale,
                            c,
                        )
                    }
                }
            }
        }
    }
    /// Draw the root-authored 7×9 console face at native scale.
    ///
    /// `font7::glyph` owns the actual glyph drawings.  This method only
    /// rasterizes the supplied rows and deliberately does not invent a
    /// fallback font, keeping the authored proof and runtime pixels aligned.
    pub fn text_readable(&mut self, s: &str, x: i32, y: i32, c: Color) {
        for (i, ch) in s.to_ascii_uppercase().chars().enumerate() {
            let bits = match ch {
                ',' => [0, 0, 0, 0, 0, 0, 12, 12, 8],
                ';' => [0, 12, 12, 0, 0, 0, 12, 12, 8],
                '\'' => [12, 12, 8, 0, 0, 0, 0, 0, 0],
                _ => crate::font7::glyph(ch),
            };
            for (row, mask) in bits.iter().enumerate() {
                for col in 0..7 {
                    if mask & (1 << (6 - col)) != 0 {
                        self.pixel(x + i as i32 * READABLE_ADVANCE + col, y + row as i32, c);
                    }
                }
            }
        }
    }
    /// UI text uses an independent integer scale, never the world zoom.
    pub fn text_readable_scaled(&mut self, s: &str, x: i32, y: i32, c: Color, scale: i32) {
        for (i, ch) in s.to_ascii_uppercase().chars().enumerate() {
            let bits = crate::font7::glyph(ch);
            for (row, mask) in bits.iter().enumerate() {
                for col in 0..7 {
                    if mask & (1 << (6 - col)) != 0 {
                        self.rect(
                            x + (i as i32 * 8 + col) * scale,
                            y + row as i32 * scale,
                            scale,
                            scale,
                            c,
                        );
                    }
                }
            }
        }
    }
    /// Copy every source texel into a whole-number block. Edge clipping is
    /// permitted; reduction and fractional scaling are intentionally absent.
    pub fn blit_integer(&mut self, source: &Canvas, x: i32, y: i32, scale: i32) {
        assert!(scale >= 1);
        for sy in 0..source.height as i32 {
            for sx in 0..source.width as i32 {
                let at = ((sy as u32 * source.width + sx as u32) * 4) as usize;
                let color: Color = source.pixels[at..at + 4].try_into().unwrap();
                // Fully clear source pixels leave the destination as it is.
                if color[3] == 0 {
                    continue;
                }
                self.rect(x + sx * scale, y + sy * scale, scale, scale, color);
            }
        }
    }
    /// Associated width helper for call sites that only have a `Canvas`.
    #[allow(dead_code)]
    pub fn text_readable_width(s: &str) -> i32 {
        text_readable_width(s)
    }
    #[allow(dead_code)]
    pub fn centered_readable(&mut self, s: &str, cx: i32, y: i32, c: Color) {
        self.text_readable(s, cx - text_readable_width(s) / 2, y, c)
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        image::save_buffer(
            path,
            &self.pixels,
            self.width,
            self.height,
            image::ColorType::Rgba8,
        )
        .map_err(|e| e.to_string())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Sprite {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub anchor_x: i32,
    pub anchor_y: i32,
    /// Authored nozzle offset from the ground anchor on firing poses.
    #[serde(default)]
    pub muzzle: Option<[i32; 2]>,
    /// First nontransparent row, derived from the validated texture at load time.
    #[serde(skip)]
    pub opaque_top: u32,
}
#[derive(Deserialize)]
struct Manifest {
    width: u32,
    height: u32,
    sprites: BTreeMap<String, Sprite>,
}
/// The baked machine shadow roles.  Verified on the atlas: they never occur
/// in a machine body above the anchor row.
pub const SHADOW_CAST: Color = [101, 94, 80, 255];
pub const SHADOW_CONTACT: Color = [51, 51, 47, 255];
/// Reflections reach at most this many rows below the anchor.
pub const WADING_DEPTH: i32 = 26;
pub struct Atlas {
    pub sprites: BTreeMap<String, Sprite>,
    pixels: Vec<u8>,
    width: u32,
}
impl Atlas {
    /// Read one native texel from an already resolved sprite rectangle.
    /// Presentation masks use this to avoid allocating full-screen scratch
    /// canvases for small water modules or debris stamps.
    pub fn sample(&self, sprite: &Sprite, x: u32, y: u32) -> Option<Color> {
        if x >= sprite.w || y >= sprite.h {
            return None;
        }
        let px = sprite.x.checked_add(x)?;
        let py = sprite.y.checked_add(y)?;
        if px >= self.width {
            return None;
        }
        let i = ((u64::from(py) * u64::from(self.width) + u64::from(px)) * 4) as usize;
        self.pixels.get(i..i.checked_add(4)?)?.try_into().ok()
    }
    /// One sprite's texels, row by row, with its width and height.
    pub fn sprite_rgba(&self, name: &str) -> Option<(Vec<u8>, u32, u32)> {
        let sprite = self.sprites.get(name)?;
        let mut rgba = Vec::with_capacity((sprite.w * sprite.h * 4) as usize);
        for y in 0..sprite.h {
            for x in 0..sprite.w {
                rgba.extend_from_slice(&self.sample(sprite, x, y)?);
            }
        }
        Some((rgba, sprite.w, sprite.h))
    }
    pub fn load(base: &Path) -> Result<Self, String> {
        let mut manifest: Manifest = serde_json::from_slice(
            &std::fs::read(base.join("art/exports/game-assets.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        if manifest.width == 0
            || manifest.height == 0
            || manifest.width > 4096
            || manifest.height > 8192
            // The v16 atlas is 4096x8192: twice the v15 pixel ceiling, still
            // one texture the compositor holds in memory.
            || u64::from(manifest.width) * u64::from(manifest.height) > 4096 * 8192
            || manifest.sprites.len() > 4096
        {
            return Err("Atlas exceeds the supported content bounds".into());
        }
        for key in [
            "hook",
            "riveter",
            "bulwark",
            "sounder",
            "wick",
            "skipper",
            "reedguard",
            "loom",
            "union_hq",
            "assembly_hq",
            "union_works",
            "assembly_works",
            "dropoff",
            "condenser",
            "tower",
        ] {
            if !manifest.sprites.contains_key(key) {
                return Err(format!("Required sprite missing: {key}"));
            }
        }
        let image = image::open(base.join("art/exports/game-assets.png"))
            .map_err(|e| e.to_string())?
            .into_rgba8();
        if image.width() != manifest.width || image.height() != manifest.height {
            return Err("Atlas dimensions do not match manifest".into());
        }
        for (name, s) in &manifest.sprites {
            if s.w == 0
                || s.h == 0
                || s.x.checked_add(s.w).is_none_or(|x| x > image.width())
                || s.y.checked_add(s.h).is_none_or(|y| y > image.height())
            {
                return Err(format!("Invalid atlas rectangle: {name}"));
            }
            if let Some([dx, dy]) = s.muzzle {
                let x = i64::from(dx) + i64::from(s.anchor_x);
                let y = i64::from(dy) + i64::from(s.anchor_y);
                if x < 0 || y < 0 || x >= i64::from(s.w) || y >= i64::from(s.h) {
                    return Err(format!("Invalid authored muzzle: {name}"));
                }
            }
        }
        for s in manifest.sprites.values_mut() {
            s.opaque_top = (0..s.h)
                .find(|y| (0..s.w).any(|x| image.get_pixel(s.x + x, s.y + y)[3] != 0))
                .unwrap_or(s.h);
        }
        let mut atlas = Self {
            sprites: manifest.sprites,
            pixels: image.into_raw(),
            width: manifest.width,
        };
        atlas.merge_ui_icons(base, manifest.height)?;
        Ok(atlas)
    }
    /// The command-card icons live in their own small sheet
    /// (tools/ui_icons), appended below the main atlas rows at load so the
    /// two can be rebuilt apart.  A missing sheet leaves the text buttons.
    fn merge_ui_icons(&mut self, base: &Path, top: u32) -> Result<(), String> {
        let Ok(bytes) = std::fs::read(base.join("art/exports/ui-icons.json")) else {
            return Ok(());
        };
        let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let image = image::open(base.join("art/exports/ui-icons.png"))
            .map_err(|e| e.to_string())?
            .into_rgba8();
        if image.width() != manifest.width
            || image.height() != manifest.height
            || manifest.width > self.width
            || manifest.height > 1024
        {
            return Err("Icon sheet does not match its manifest".into());
        }
        for (name, s) in &manifest.sprites {
            if s.w == 0
                || s.h == 0
                || s.x.checked_add(s.w).is_none_or(|x| x > image.width())
                || s.y.checked_add(s.h).is_none_or(|y| y > image.height())
            {
                return Err(format!("Invalid icon rectangle: {name}"));
            }
        }
        let row = self.width as usize * 4;
        for y in 0..image.height() {
            let start = self.pixels.len();
            self.pixels.resize(start + row, 0);
            let src = y as usize * image.width() as usize * 4;
            let len = image.width() as usize * 4;
            self.pixels[start..start + len].copy_from_slice(&image.as_raw()[src..src + len]);
        }
        for (name, mut s) in manifest.sprites {
            s.y += top;
            s.opaque_top = 0;
            self.sprites.insert(name, s);
        }
        Ok(())
    }
    pub fn draw(&self, canvas: &mut Canvas, key: &str, x: i32, y: i32, dim: bool) -> bool {
        let Some(s) = self.sprites.get(key) else {
            return false;
        };
        let left = x - s.anchor_x;
        let top = y - s.anchor_y;
        let x0 = (-left).max(0).min(s.w as i32) as u32;
        let x1 = (canvas.width() as i32 - left).clamp(0, s.w as i32) as u32;
        let y0 = (-top).max(0).min(s.h as i32) as u32;
        let y1 = (canvas.height() as i32 - top).clamp(0, s.h as i32) as u32;
        for yy in y0..y1 {
            for xx in x0..x1 {
                let i = (((s.y + yy) * self.width + s.x + xx) * 4) as usize;
                let mut c: Color = self.pixels[i..i + 4]
                    .try_into()
                    .expect("validated RGBA pixel");
                if c[3] == 0 {
                    continue;
                }
                if dim {
                    c = crate::chart_surface::chart_color(c);
                }
                canvas.pixel(left + xx as i32, top + yy as i32, c);
            }
        }
        true
    }
    /// Draw a machine standing in water.  The baked ground shadow (the
    /// `cast` and `contact` roles at or below the anchor row, which never
    /// occur in a machine body above it) is skipped, and the body is mirrored
    /// below the anchor in the sea ramp with broken rows, only where
    /// `is_water` says the screen pixel lies on a flooded cell.  Picking uses
    /// the unmodified sprite, so the reflection is never selectable.
    pub fn draw_wading(
        &self,
        canvas: &mut Canvas,
        key: &str,
        x: i32,
        y: i32,
        is_water: &dyn Fn(i32, i32) -> bool,
    ) -> bool {
        let Some(s) = self.sprites.get(key) else {
            return false;
        };
        let left = x - s.anchor_x;
        let top = y - s.anchor_y;
        for yy in 0..s.h {
            for xx in 0..s.w {
                let i = (((s.y + yy) * self.width + s.x + xx) * 4) as usize;
                let c: Color = self.pixels[i..i + 4]
                    .try_into()
                    .expect("validated RGBA pixel");
                if c[3] == 0 {
                    continue;
                }
                let below_anchor = yy as i32 >= s.anchor_y;
                // The shadow roles never occur in a machine body (the source
                // verifier proves it), so they are skipped wherever they lie.
                if c == SHADOW_CAST || c == SHADOW_CONTACT {
                    continue;
                }
                let px = left + xx as i32;
                let py = top + yy as i32;
                canvas.pixel(px, py, c);
                if below_anchor {
                    continue;
                }
                // Mirror about the anchor row.  Rows break in pairs offset by
                // column so the reflection never forms a second solid body.
                let depth = s.anchor_y - yy as i32;
                if depth > WADING_DEPTH {
                    continue;
                }
                let ry = y + depth;
                if (ry + px / 2).rem_euclid(2) != 0 || !is_water(px, ry) {
                    continue;
                }
                let luma = (u32::from(c[0]) * 3 + u32::from(c[1]) * 6 + u32::from(c[2])) / 10;
                let tint = if luma > 170 {
                    [128, 168, 166, 120]
                } else if luma > 95 {
                    [77, 124, 130, 150]
                } else {
                    [33, 63, 75, 200]
                };
                canvas.pixel(px, ry, tint);
            }
        }
        true
    }
    /// Draw a one-pixel outline around the body of `key` (shadows excluded)
    /// in `color`, on the transparent texels that touch a body texel.  Used
    /// to mark every enemy machine and building so ownership reads at a
    /// glance regardless of faction material.
    pub fn draw_outline(
        &self,
        canvas: &mut Canvas,
        key: &str,
        x: i32,
        y: i32,
        scale: crate::occlusion::PixelScale,
        color: Color,
    ) -> bool {
        let Some(s) = self.sprites.get(key) else {
            return false;
        };
        let n = u32::from(scale.numerator);
        let d = u32::from(scale.denominator);
        let w = (s.w * n).div_ceil(d) as i32;
        let h = (s.h * n).div_ceil(d) as i32;
        let left = x - scale.apply(s.anchor_x);
        let top = y - scale.apply(s.anchor_y);
        let body = |xx: i32, yy: i32| -> bool {
            if xx < 0 || yy < 0 || xx >= w || yy >= h {
                return false;
            }
            self.sample(s, xx as u32 * d / n, yy as u32 * d / n)
                .is_some_and(|c| c[3] != 0 && c != SHADOW_CAST && c != SHADOW_CONTACT)
        };
        for yy in -1..=h {
            for xx in -1..=w {
                if body(xx, yy) {
                    continue;
                }
                if body(xx - 1, yy) || body(xx + 1, yy) || body(xx, yy - 1) || body(xx, yy + 1) {
                    canvas.pixel(left + xx, top + yy, color);
                }
            }
        }
        true
    }
    /// Draw a tinted, half-transparent ghost of `key` only through the opaque
    /// pixels of `occluder` (drawn at its own anchor and scale), so a machine
    /// hidden behind a building stays visible where the building covers it.
    /// Baked shadows are not ghosted.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_ghost_through(
        &self,
        canvas: &mut Canvas,
        key: &str,
        x: i32,
        y: i32,
        scale: crate::occlusion::PixelScale,
        occluder: &str,
        ox: i32,
        oy: i32,
        occluder_scale: crate::occlusion::PixelScale,
        tint: Color,
    ) -> bool {
        let Some(s) = self.sprites.get(key) else {
            return false;
        };
        if !self.sprites.contains_key(occluder) {
            return false;
        }
        let n = u32::from(scale.numerator);
        let d = u32::from(scale.denominator);
        let w = (s.w * n).div_ceil(d);
        let h = (s.h * n).div_ceil(d);
        let left = x - scale.apply(s.anchor_x);
        let top = y - scale.apply(s.anchor_y);
        let mix = |a: u8, b: u8| ((u16::from(a) + u16::from(b)) / 2) as u8;
        for yy in 0..h {
            for xx in 0..w {
                let Some(c) = self.sample(s, xx * d / n, yy * d / n).filter(|c| c[3] != 0) else {
                    continue;
                };
                if c == SHADOW_CAST || c == SHADOW_CONTACT {
                    continue;
                }
                let px = left + xx as i32;
                let py = top + yy as i32;
                if !self.hit_scaled(occluder, px - ox, py - oy, occluder_scale) {
                    continue;
                }
                canvas.pixel(
                    px,
                    py,
                    [
                        mix(c[0], tint[0]),
                        mix(c[1], tint[1]),
                        mix(c[2], tint[2]),
                        170,
                    ],
                );
            }
        }
        true
    }
    /// Draw `key` half transparent, for a building still being placed.
    /// Baked shadow pixels are skipped; `tint` reddens a refused site.
    pub fn draw_ghost(
        &self,
        canvas: &mut Canvas,
        key: &str,
        x: i32,
        y: i32,
        scale: crate::occlusion::PixelScale,
        tint: Option<Color>,
    ) -> bool {
        let Some(s) = self.sprites.get(key) else {
            return false;
        };
        let n = u32::from(scale.numerator);
        let d = u32::from(scale.denominator);
        let w = (s.w * n).div_ceil(d);
        let h = (s.h * n).div_ceil(d);
        let left = x - scale.apply(s.anchor_x);
        let top = y - scale.apply(s.anchor_y);
        let mix = |a: u8, b: u8| ((u16::from(a) + u16::from(b)) / 2) as u8;
        for yy in 0..h {
            for xx in 0..w {
                let Some(c) = self.sample(s, xx * d / n, yy * d / n).filter(|c| c[3] != 0) else {
                    continue;
                };
                if c == SHADOW_CAST || c == SHADOW_CONTACT {
                    continue;
                }
                let c = match tint {
                    Some(t) => [mix(c[0], t[0]), mix(c[1], t[1]), mix(c[2], t[2]), 140],
                    None => [c[0], c[1], c[2], 128],
                };
                canvas.pixel(left + xx as i32, top + yy as i32, c);
            }
        }
        true
    }
    pub fn hit(&self, key: &str, dx: i32, dy: i32) -> bool {
        let Some(sprite) = self.sprites.get(key) else {
            return false;
        };
        let x = dx + sprite.anchor_x;
        let y = dy + sprite.anchor_y;
        if x < 0 || y < 0 || x >= sprite.w as i32 || y >= sprite.h as i32 {
            return false;
        }
        let index = (((sprite.y + y as u32) * self.width + sprite.x + x as u32) * 4 + 3) as usize;
        self.pixels[index] >= 128
    }
    pub fn draw_scaled(
        &self,
        canvas: &mut Canvas,
        key: &str,
        x: i32,
        y: i32,
        dim: bool,
        scale: crate::occlusion::PixelScale,
    ) -> bool {
        if scale == crate::occlusion::PixelScale::ONE {
            return self.draw(canvas, key, x, y, dim);
        }
        let Some(s) = self.sprites.get(key) else {
            return false;
        };
        let n = u32::from(scale.numerator);
        let d = u32::from(scale.denominator);
        let w = (s.w * n).div_ceil(d);
        let h = (s.h * n).div_ceil(d);
        let left = x - scale.apply(s.anchor_x);
        let top = y - scale.apply(s.anchor_y);
        for yy in 0..h {
            for xx in 0..w {
                if let Some(mut color) =
                    self.sample(s, xx * d / n, yy * d / n).filter(|c| c[3] != 0)
                {
                    if dim {
                        color = crate::chart_surface::chart_color(color)
                    }
                    canvas.pixel(left + xx as i32, top + yy as i32, color);
                }
            }
        }
        true
    }
    pub fn hit_scaled(
        &self,
        key: &str,
        dx: i32,
        dy: i32,
        scale: crate::occlusion::PixelScale,
    ) -> bool {
        if scale == crate::occlusion::PixelScale::ONE {
            return self.hit(key, dx, dy);
        }
        let Some(s) = self.sprites.get(key) else {
            return false;
        };
        let x = dx + scale.apply(s.anchor_x);
        let y = dy + scale.apply(s.anchor_y);
        if x < 0 || y < 0 {
            return false;
        }
        let n = u32::from(scale.numerator);
        let d = u32::from(scale.denominator);
        if x as u32 >= (s.w * n).div_ceil(d) || y as u32 >= (s.h * n).div_ceil(d) {
            return false;
        }
        self.sample(s, x as u32 * d / n, y as u32 * d / n)
            .is_some_and(|c| c[3] >= 128)
    }
}

pub(crate) fn glyph(c: char) -> [u8; 7] {
    match c {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [15, 16, 16, 16, 16, 16, 15],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [15, 16, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 18, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        ':' => [0, 4, 4, 0, 4, 4, 0],
        '/' => [1, 1, 2, 4, 8, 16, 16],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '.' => [0, 0, 0, 0, 0, 4, 4],
        ',' => [0, 0, 0, 0, 4, 4, 8],
        // "Choose clear ground; Esc cancels" lost its stop (trial 11).
        ';' => [0, 4, 4, 0, 4, 4, 8],
        '+' => [0, 4, 4, 31, 4, 4, 0],
        '%' => [17, 2, 4, 8, 16, 17, 0],
        '&' => [12, 18, 18, 12, 21, 18, 13],
        '(' => [2, 4, 8, 8, 8, 4, 2],
        ')' => [8, 4, 2, 2, 2, 4, 8],
        '[' => [14, 8, 8, 8, 8, 8, 14],
        ']' => [14, 2, 2, 2, 2, 2, 14],
        '!' => [4, 4, 4, 4, 4, 0, 4],
        '?' => [14, 17, 1, 2, 4, 0, 4],
        '\'' => [4, 4, 8, 0, 0, 0, 0],
        '>' => [16, 8, 4, 2, 4, 8, 16],
        '<' => [1, 2, 4, 8, 4, 2, 1],
        '=' => [0, 0, 31, 0, 31, 0, 0],
        '_' => [0, 0, 0, 0, 0, 0, 31],
        _ => [0; 7],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_surface_keeps_the_authored_dimensions_and_stride() {
        let canvas = Canvas::default();
        assert_eq!((canvas.width(), canvas.height()), (CANVAS_W, CANVAS_H));
        assert_eq!(canvas.pixels.len(), (CANVAS_W * CANVAS_H * 4) as usize);
    }

    #[test]
    fn wide_surface_uses_its_own_bounds_and_row_stride() {
        let mut canvas = Canvas::new(1280, 552);
        let marker = [241, 17, 33, 255];
        canvas.pixel(700, 300, marker);
        canvas.pixel(1279, 551, marker);
        canvas.rect(1278, 550, 4, 4, marker);

        let index = |x: u32, y: u32| ((y * canvas.width() + x) * 4) as usize;
        assert_eq!(
            &canvas.pixels[index(700, 300)..index(700, 300) + 4],
            &marker
        );
        assert_eq!(
            &canvas.pixels[index(1279, 551)..index(1279, 551) + 4],
            &marker
        );
        assert_eq!(
            &canvas.pixels[index(1278, 550)..index(1278, 550) + 4],
            &marker
        );
        assert_eq!(
            &canvas.pixels[index(1279, 550)..index(1279, 550) + 4],
            &marker
        );
        // The old 640×360 edge is no longer a clipping boundary.
        assert_eq!(
            &canvas.pixels[index(700, 300)..index(700, 300) + 4],
            &marker
        );
    }

    #[test]
    fn resize_changes_dimensions_and_clears_the_new_surface() {
        let mut canvas = Canvas::default();
        canvas.pixel(639, 359, RED);
        canvas.resize(17, 9);
        assert_eq!((canvas.width(), canvas.height()), (17, 9));
        assert_eq!(canvas.pixels.len(), 17 * 9 * 4);
        assert!(canvas.pixels.iter().all(|byte| *byte == 0));
        canvas.pixel(16, 8, RED);
        assert_eq!(
            &canvas.pixels[(16 + 8 * 17) * 4..(16 + 8 * 17) * 4 + 4],
            &RED
        );
    }

    #[test]
    #[should_panic(expected = "Canvas dimensions")]
    fn canvas_rejects_oversized_surfaces() {
        let _ = Canvas::new(MAX_CANVAS_WIDTH + 1, 1);
    }
}
