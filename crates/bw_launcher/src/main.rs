//! BRINEWAKE's launcher: a small window that checks danprice.ai/brinewake
//! for a newer game, installs it, and starts the game.
//!
//! `--headless` does the same without a window, printing each stage (for
//! testing). `BRINEWAKE_UPDATE_URL` points it at another manifest,
//! `BRINEWAKE_GAME_STORE` at another install folder, and
//! `BRINEWAKE_NO_LAUNCH=1` stops before starting the game.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use bw_launcher::{Install, MANIFEST_URLS, PAGE, Stage, Store, bundled, font, update};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

/// The canvas, in game pixels.
const W: usize = 320;
const H: usize = 180;
/// The window shows at least this long, so it does not flash.
const MIN_SHOWN: Duration = Duration::from_millis(1200);
/// A note (offline, or an update that failed) stays up this long.
const NOTE_SHOWN: Duration = Duration::from_millis(2600);

// The game's palette.
const INK: u32 = rgb(19, 31, 39);
const PANEL: u32 = rgb(28, 45, 54);
const EDGE: u32 = rgb(71, 96, 102);
const WHITE: u32 = rgb(239, 228, 197);
const MUTED: u32 = rgb(151, 169, 167);
const GOLD: u32 = rgb(236, 177, 96);
const JADE: u32 = rgb(127, 194, 164);
const RED: u32 = rgb(226, 108, 84);

const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

fn manifest_urls() -> Vec<String> {
    match std::env::var("BRINEWAKE_UPDATE_URL") {
        Ok(url) => vec![url],
        Err(_) => MANIFEST_URLS.iter().map(|u| u.to_string()).collect(),
    }
}

fn no_launch() -> bool {
    std::env::var_os("BRINEWAKE_NO_LAUNCH").is_some_and(|v| v == "1")
}

/// Start the game and leave it running on its own.
fn launch(install: &Install) -> Result<(), String> {
    if no_launch() {
        println!("would start {}", install.exe().display());
        return Ok(());
    }
    let mut command = std::process::Command::new(install.exe());
    command.current_dir(&install.dir);
    command
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("could not start the game: {e}"))
}

fn stage_line(stage: &Stage) -> String {
    match stage {
        Stage::Checking => "CHECKING FOR UPDATES".into(),
        Stage::Downloading {
            version,
            done,
            total,
        } => {
            let mb = |b: u64| b as f64 / 1_048_576.0;
            format!(
                "DOWNLOADING {version}  {:.1} / {:.1} MB",
                mb(*done),
                mb(*total)
            )
        }
        Stage::Installing { version } => format!("INSTALLING {version}"),
        Stage::Ready { install, .. } => format!("STARTING {}", install.version),
        Stage::Failed(e) => e.clone(),
    }
}

fn headless() -> i32 {
    let store = Store::default_location();
    // Downloads report every tenth.
    let mut shown = u64::MAX;
    let stage = update(&manifest_urls(), store.as_ref(), bundled(), |s| {
        if let Stage::Downloading { done, total, .. } = &s {
            let tenth = done * 10 / (*total).max(1);
            if tenth == shown {
                return;
            }
            shown = tenth;
        }
        println!("{}", stage_line(&s));
    });
    match stage {
        Stage::Ready { install, note } => {
            if let Some(note) = note {
                println!("{note}");
            }
            println!(
                "{}",
                stage_line(&Stage::Ready {
                    install: install.clone(),
                    note: None
                })
            );
            match launch(&install) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("{e}");
                    1
                }
            }
        }
        other => {
            eprintln!("{}", stage_line(&other));
            1
        }
    }
}

struct Launcher {
    stage: Arc<Mutex<Stage>>,
    started: Instant,
    /// When the final stage arrived.
    finished: Option<Instant>,
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    canvas: Vec<u32>,
}

impl Launcher {
    fn draw(&mut self) {
        let stage = self
            .stage
            .lock()
            .map(|s| s.clone())
            .unwrap_or(Stage::Checking);
        let t = self.started.elapsed().as_secs_f32();
        let c = &mut self.canvas;
        c.fill(INK);
        // The tide: three bands of water rolling under the title.
        for band in 0..3 {
            let base = 118 + band * 14;
            let color = [PANEL, EDGE, JADE][band];
            for x in 0..W {
                let phase = x as f32 * 0.06 + t * (0.9 + band as f32 * 0.35) + band as f32 * 1.7;
                let y = base as f32 + phase.sin() * (2.0 + band as f32);
                for yy in (y as usize).min(H)..H.min(base + 40) {
                    if band < 2 || yy <= y as usize + 1 {
                        c[yy * W + x] = color;
                    }
                }
            }
        }
        for y in 150..H {
            for x in 0..W {
                c[y * W + x] = INK;
            }
        }
        text(c, "BRINEWAKE", center(9, 3), 38, 3, WHITE);
        text(c, "THE TIDE DECIDES", center(16, 1), 66, 1, GOLD);
        let (line, color) = match &stage {
            Stage::Failed(_) => (stage_line(&stage), RED),
            Stage::Ready {
                note: Some(note), ..
            } => (note.clone(), MUTED),
            _ => (stage_line(&stage), WHITE),
        };
        let line: String = line.chars().take(52).collect();
        text(c, &line, center(line.len(), 1), 90, 1, color);
        // Progress: a gold bar while downloading, a sweep while checking.
        let (bx, by, bw) = (70usize, 104usize, 180usize);
        rect(c, bx, by, bw, 3, PANEL);
        match &stage {
            Stage::Downloading { done, total, .. } if *total > 0 => {
                rect(c, bx, by, (bw as u64 * done / total) as usize, 3, GOLD);
            }
            Stage::Installing { .. } | Stage::Ready { note: None, .. } => {
                rect(c, bx, by, bw, 3, GOLD)
            }
            Stage::Checking => {
                let x = ((t * 90.0) as usize) % (bw + 30);
                let start = x.saturating_sub(30);
                rect(c, bx + start, by, x.min(bw) - start.min(x.min(bw)), 3, GOLD);
            }
            _ => {}
        }
        let version = format!("LAUNCHER {}", env!("CARGO_PKG_VERSION"));
        text(c, &version, 6, H - 12, 1, EDGE);
        text(
            c,
            &PAGE.to_ascii_uppercase(),
            W - 6 - PAGE.len() * 6,
            H - 12,
            1,
            EDGE,
        );
        let (Some(window), Some(surface)) = (&self.window, &mut self.surface) else {
            return;
        };
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        if surface.resize(w, h).is_err() {
            return;
        }
        let Ok(mut buffer) = surface.buffer_mut() else {
            return;
        };
        let (sw, sh) = (size.width as usize, size.height as usize);
        let scale = (sw / W).min(sh / H).max(1);
        let (ox, oy) = (
            (sw.saturating_sub(W * scale)) / 2,
            (sh.saturating_sub(H * scale)) / 2,
        );
        buffer.fill(INK);
        for y in 0..(H * scale).min(sh) {
            let row = &self.canvas[(y / scale) * W..(y / scale + 1) * W];
            let out = &mut buffer[(oy + y) * sw..(oy + y + 1) * sw];
            for x in 0..(W * scale).min(sw - ox) {
                out[ox + x] = row[x / scale];
            }
        }
        let _ = buffer.present();
    }
}

fn center(chars: usize, scale: usize) -> usize {
    W.saturating_sub(chars * 6 * scale - scale) / 2
}

fn rect(c: &mut [u32], x: usize, y: usize, w: usize, h: usize, color: u32) {
    for yy in y..(y + h).min(H) {
        for xx in x..(x + w).min(W) {
            c[yy * W + xx] = color;
        }
    }
}

fn text(c: &mut [u32], s: &str, x: usize, y: usize, scale: usize, color: u32) {
    for (i, ch) in s.to_ascii_uppercase().chars().enumerate() {
        for (row, mask) in font::glyph(ch).iter().enumerate() {
            for col in 0..5 {
                if mask & (1 << (4 - col)) != 0 {
                    rect(
                        c,
                        x + (i * 6 + col) * scale,
                        y + row * scale,
                        scale,
                        scale,
                        color,
                    );
                }
            }
        }
    }
}

impl ApplicationHandler for Launcher {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("BRINEWAKE")
            .with_inner_size(LogicalSize::new((W * 3) as f64, (H * 3) as f64))
            .with_resizable(false);
        let Ok(window) = event_loop.create_window(attributes) else {
            event_loop.exit();
            return;
        };
        let window = Rc::new(window);
        let surface = softbuffer::Context::new(window.clone())
            .and_then(|context| softbuffer::Surface::new(&context, window.clone()));
        self.surface = surface.ok();
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            // Closing the window cancels: nothing starts.
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.draw(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let stage = self
            .stage
            .lock()
            .map(|s| s.clone())
            .unwrap_or(Stage::Checking);
        let done = matches!(stage, Stage::Ready { .. } | Stage::Failed(_));
        if done && self.finished.is_none() {
            self.finished = Some(Instant::now());
        }
        if let Some(finished) = self.finished {
            let hold = match &stage {
                Stage::Ready { note: None, .. } => Duration::ZERO,
                _ => NOTE_SHOWN,
            };
            if self.started.elapsed() >= MIN_SHOWN && finished.elapsed() >= hold {
                match stage {
                    Stage::Ready { install, .. } => {
                        if let Err(e) = launch(&install) {
                            *self.stage.lock().unwrap() = Stage::Failed(e.to_ascii_uppercase());
                            self.finished = Some(Instant::now() + Duration::from_secs(4));
                            return;
                        }
                        event_loop.exit();
                    }
                    _ => event_loop.exit(),
                }
                return;
            }
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(33),
        ));
    }
}

fn main() {
    if std::env::args().any(|a| a == "--headless") {
        std::process::exit(headless());
    }
    let stage = Arc::new(Mutex::new(Stage::Checking));
    let shared = Arc::clone(&stage);
    std::thread::spawn(move || {
        let store = Store::default_location();
        let last = update(&manifest_urls(), store.as_ref(), bundled(), |s| {
            if let Ok(mut slot) = shared.lock() {
                *slot = s;
            }
        });
        if let Ok(mut slot) = shared.lock() {
            *slot = last;
        }
    });
    let Ok(event_loop) = EventLoop::new() else {
        std::process::exit(headless());
    };
    let mut app = Launcher {
        stage,
        started: Instant::now(),
        finished: None,
        window: None,
        surface: None,
        canvas: vec![INK; W * H],
    };
    let _ = event_loop.run_app(&mut app);
}
