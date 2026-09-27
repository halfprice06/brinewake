// A Windows player double-clicks the game: no console window behind it.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod agent;
mod ambience;
mod art_review;
mod art_v15_review;
mod art_v16_review;
mod artistry;
mod artistry_review;
mod audio;
mod audio_scene;
mod camera_review;
mod canvas;
mod chart_memory;
mod chart_review;
mod chart_surface;
mod coach;
mod compact_review;
mod console;
mod controls;
mod damage;
mod daylight;
mod depth_review;
mod dock_log;
mod dock_qol;
mod dock_ui;
mod field_alerts;
mod field_labels;
mod field_manual;
mod field_marks_review;
mod font7;
mod game;
mod glint_fx;
mod gpu;
mod ground_light;
mod ground_marks;
mod guide_and_chart;
mod home_backdrop;
mod hover_card;
mod lane_memory;
mod lane_wall;
mod lean_hud;
mod match_summary;
mod matchup;
mod menus;
mod minimap_chart;
mod music;
mod music_timeline;
mod native_review;
mod native_ui;
mod net;
mod net_ui;
mod observer;
mod occlusion;
mod onboarding;
mod playback;
mod pointer;
mod polish_journeys;
mod presentation;
mod production_qol;
mod qol;
mod qol_v2_review;
mod qol_v3_review;
mod readability_review;
mod render_review;
mod rules20_words;
mod rules21_words;
mod rules22_words;
mod score;
mod score_confluence;
mod score_songs;
mod score_undertow;
mod seats;
mod sfx;
mod sound_director;
mod spectator;
mod station3;
mod synth;
mod tactics;
mod tempo;
#[cfg(test)]
mod tests;
mod tidal_traces;
mod tide_cues;
mod tide_gauge;
mod tide_v3;
mod trailer;
mod transports;
mod trial10_words;
mod trial11_controls;
mod trial11_words;
mod trial12_field;
mod trial12_hold;
mod trial12_orders;
mod trial5_fixes;
mod trial6_fixes;
mod trial7_fixes;
mod trial8_fixes;
mod trial8_hud;
mod ux;
mod ux_review;
mod watchdog;
mod wind;
mod zoom;

use game::{Game, Screen};
use gpu::Gpu;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize, Position};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};
use winit::window::{CustomCursor, Window, WindowId};

struct App {
    game: Game,
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    modifiers: ModifiersState,
    keys: BTreeSet<String>,
    last_time: Instant,
    accumulator: f64,
    clock_mode: (Screen, tempo::GameSpeed, bool),
    frame_due: Instant,
    error: Option<String>,
    size: (u32, u32),
    quit_after: Option<Duration>,
    started: Instant,
    capture: Option<PathBuf>,
    middle: Option<(i32, i32)>,
    focused: bool,
    /// The direction the view is edge-scrolling this frame, for the pointer.
    edge: (i32, i32),
    /// Pointer pictures made so far, by shape and scale.
    cursors: std::collections::HashMap<(pointer::PointerKind, u32), CustomCursor>,
    cursor_shown: Option<(pointer::PointerKind, u32)>,
    /// A line on stderr for a tick that runs long (debug builds, or
    /// `BRINEWAKE_WATCHDOG_MS`).
    watchdog: watchdog::Watchdog,
}
impl App {
    /// Give the window the pointer for what it rests on, making each
    /// picture once.
    fn show_pointer(&mut self, event_loop: &ActiveEventLoop) {
        let Some(window) = &self.window else {
            return;
        };
        let key = (self.game.pointer_kind(self.edge), self.game.pointer_scale());
        if self.cursor_shown == Some(key) {
            return;
        }
        let cursor = match self.cursors.get(&key) {
            Some(cursor) => cursor.clone(),
            None => {
                let img = pointer::image(key.0, key.1);
                let Ok(source) = CustomCursor::from_rgba(
                    img.rgba,
                    img.w as u16,
                    img.h as u16,
                    img.hot_x as u16,
                    img.hot_y as u16,
                ) else {
                    return;
                };
                let cursor = event_loop.create_custom_cursor(source);
                self.cursors.insert(key, cursor.clone());
                cursor
            }
        };
        window.set_cursor(cursor);
        self.cursor_shown = Some(key);
    }
    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
        self.keys.clear();
        self.modifiers = ModifiersState::empty();
        self.middle = None;
        tempo::reset_accumulator(&mut self.accumulator);
        self.last_time = Instant::now();
        self.game.ux.pointer_shift = false;
        if !focused {
            self.game.focus_lost();
        }
    }
    fn new(
        mut game: Game,
        size: (u32, u32),
        quit_after: Option<Duration>,
        capture: Option<PathBuf>,
    ) -> Self {
        // Runtime defaults wider; renderer fixtures can explicitly keep their
        // neutral native projection through Game's standalone constructor.
        game.zoom = zoom::Zoom::from_preference(game.ux.preferences.world_zoom);
        game.resize_view(size.0, size.1);
        let clock_mode = (
            game.screen,
            game.ux.preferences.game_speed,
            game.world.outcome.is_some() || game.ux.practice_review,
        );
        Self {
            clock_mode,
            game,
            window: None,
            gpu: None,
            modifiers: ModifiersState::default(),
            keys: BTreeSet::new(),
            last_time: Instant::now(),
            accumulator: 0.0,
            frame_due: Instant::now(),
            error: None,
            size,
            quit_after,
            started: Instant::now(),
            capture,
            middle: None,
            focused: true,
            edge: (0, 0),
            cursors: Default::default(),
            cursor_shown: None,
            watchdog: watchdog::Watchdog::from_env(),
        }
    }
    fn advance_clocks(&mut self, elapsed: f64) {
        let terminal = self.game.world.outcome.is_some() || self.game.ux.practice_review;
        let mode = (
            self.game.screen,
            self.game.ux.preferences.game_speed,
            terminal,
        );
        let elapsed = if mode != self.clock_mode {
            self.clock_mode = mode;
            tempo::reset_accumulator(&mut self.accumulator);
            0.0
        } else {
            elapsed
        };
        // A network match runs on while this seat reads a menu: the other
        // seats are still playing.
        let stepping = self.game.screen == Screen::Match || self.game.session_steps_on();
        if stepping {
            // A lockstep match runs at the shared normal pace.
            let speed = if terminal || self.game.session.is_some() {
                tempo::GameSpeed::Normal
            } else {
                mode.1
            };
            // An observer that has fallen behind a live match catches up.
            let pace = self
                .game
                .playback
                .as_ref()
                .map_or(1, |p| p.catch_up_factor());
            for _ in 0..tempo::schedule_ticks(&mut self.accumulator, elapsed, speed) * pace {
                self.watchdog.tick(&mut self.game);
                let now_terminal =
                    self.game.world.outcome.is_some() || self.game.ux.practice_review;
                if now_terminal != terminal {
                    // A victory can occur inside a catch-up burst. Remaining
                    // ticks were scheduled at the old field pace; discard them
                    // and begin the aftermath's normal clock next frame.
                    tempo::reset_accumulator(&mut self.accumulator);
                    self.clock_mode = (self.game.screen, mode.1, now_terminal);
                    break;
                }
            }
        } else if self.game.screen == Screen::Help && self.game.ux.help_page == 4 {
            for _ in
                0..tempo::schedule_ticks(&mut self.accumulator, elapsed, tempo::GameSpeed::Normal)
            {
                self.game.ux.manual.advance();
            }
        } else {
            tempo::reset_accumulator(&mut self.accumulator);
        }
    }
    fn failure(&mut self, event_loop: &ActiveEventLoop, error: String) {
        eprintln!("BRINEWAKE: {error}");
        self.error = Some(error);
        event_loop.exit();
    }
    fn draw(&mut self, event_loop: &ActiveEventLoop) {
        if self.game.ux.quit_requested {
            event_loop.exit();
            return;
        }
        if let Some(fullscreen) = self.game.ux.fullscreen_request.take()
            && let Some(window) = &self.window
        {
            window.set_fullscreen(if fullscreen {
                Some(winit::window::Fullscreen::Borderless(None))
            } else {
                None
            });
        }
        if let Some(window) = &self.window {
            // The macOS window control can change display mode independently
            // of F11/Settings. Reflect the actual mode in both UI and storage.
            let fullscreen = window.fullscreen().is_some();
            let changed = self.game.ux.fullscreen != fullscreen
                || self.game.ux.preferences.fullscreen != fullscreen;
            self.game.ux.fullscreen = fullscreen;
            self.game.ux.preferences.fullscreen = fullscreen;
            if changed && self.game.ux.persist(&self.game.data_dir).is_err() {
                self.game
                    .notify("Display changed. Settings could not be saved.");
            }
        }
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_time).as_secs_f64();
        self.last_time = now;
        self.game.frame_update();
        self.game.update_sound();
        self.advance_clocks(elapsed);
        self.edge = (0, 0);
        if self.game.screen == Screen::Match
            && self.focused
            && !self.game.ux.keyboard_navigation
            && !self.game.ux.minimap_drag
        {
            let mut dx = 0;
            let mut dy = 0;
            // Edge scrolling uses the whole window edge, header and dock
            // included, as the classic RTS convention does; the permanent
            // interface must not leave dead zones at the top and bottom.
            let (ex, ey) = if self.game.ux.preferences.edge_scroll && self.middle.is_none() {
                edge_scroll_delta(self.game.cursor, self.size, 4 * self.game.ui_scale())
            } else {
                (0, 0)
            };
            self.edge = (ex, ey);
            if self.keys.contains("ArrowLeft") || ex < 0 {
                dx -= 8
            }
            if self.keys.contains("ArrowRight") || ex > 0 {
                dx += 8
            }
            if self.keys.contains("ArrowUp") || ey < 0 {
                dy -= 8
            }
            if self.keys.contains("ArrowDown") || ey > 0 {
                dy += 8
            }
            self.game
                .pan(dx * self.game.ui_scale(), dy * self.game.ui_scale())
        }
        self.game.resize_view(self.size.0, self.size.1);
        self.game.render();
        self.show_pointer(event_loop);
        if let Some(gpu) = &mut self.gpu
            && let Err(e) = gpu.present(&self.game.canvas.pixels)
        {
            self.failure(event_loop, e);
            return;
        }
        if self
            .quit_after
            .is_some_and(|duration| self.started.elapsed() >= duration)
        {
            if let Some(path) = self.capture.take()
                && let Err(e) = self.game.canvas.save(&path)
            {
                eprintln!("Capture failed: {e}")
            }
            println!(
                "Native smoke session ended at simulation tick {}",
                self.game.world.tick
            );
            event_loop.exit();
            return;
        }
        self.frame_due = now + Duration::from_nanos(16_666_667);
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.frame_due));
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("BRINEWAKE — The Split Basin")
            .with_inner_size(PhysicalSize::new(self.size.0, self.size.1))
            .with_min_inner_size(PhysicalSize::new(960, 540))
            .with_max_inner_size(PhysicalSize::new(8192, 8192))
            .with_position(Position::Physical(PhysicalPosition::new(90, 70)))
            .with_visible(false);
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                self.failure(event_loop, e.to_string());
                return;
            }
        };
        if let Some(icon) = self.game.atlas.as_ref().and_then(window_icon) {
            window.set_window_icon(Some(icon));
        }
        let actual_size = window.inner_size();
        self.size = (actual_size.width, actual_size.height);
        self.game.display_scale = window.scale_factor().round().clamp(1.0, 4.0) as i32;
        match Gpu::new(window.clone()) {
            Ok(gpu) => {
                println!("{}", gpu.info());
                self.gpu = Some(gpu)
            }
            Err(e) => {
                self.failure(event_loop, e);
                return;
            }
        }
        match audio::Audio::new() {
            Ok(a) => {
                a.set_muted(self.game.muted);
                a.set_levels(
                    self.game.ux.preferences.effects_volume,
                    self.game.ux.preferences.ambience_volume,
                    self.game.ux.preferences.music_volume,
                );
                self.game.audio = Some(a);
            }
            Err(e) => eprintln!("Audio unavailable; game continues: {e}"),
        }
        window.set_visible(true);
        window.request_redraw();
        self.window = Some(window);
        self.last_time = Instant::now();
        self.started = Instant::now();
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.window.as_ref().is_none_or(|w| w.id() != id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => {
                self.game.action(game::Action::Quit);
                if self.game.ux.quit_requested {
                    event_loop.exit();
                } else if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::Resized(size) => {
                self.size = (size.width, size.height);
                self.game.resize_view(size.width, size.height);
                if let Some(g) = &mut self.gpu {
                    g.resize(size.width, size.height)
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.game.display_scale = scale_factor.round().clamp(1.0, 4.0) as i32;
                if let Some(w) = &self.window {
                    let size = w.inner_size();
                    self.size = (size.width, size.height);
                    self.game.resize_view(size.width, size.height);
                    if let Some(g) = &mut self.gpu {
                        g.resize(size.width, size.height)
                    }
                }
            }
            WindowEvent::Focused(f) => {
                self.set_focus(f);
            }
            WindowEvent::ModifiersChanged(m) => {
                let was_shift = self.modifiers.shift_key();
                self.modifiers = m.state();
                self.game.ux.pointer_shift = self.modifiers.shift_key();
                if was_shift && !self.modifiers.shift_key() {
                    self.game.shift_released();
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.game.pointer_left();
                self.middle = None;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if self.size.0 > 0
                    && self.size.1 > 0
                    && self
                        .game
                        .world_pointer_allowed(self.game.cursor.0, self.game.cursor.1)
                {
                    let amount = match delta {
                        MouseScrollDelta::LineDelta(_, y) => f64::from(y) * 60.0,
                        MouseScrollDelta::PixelDelta(p) => p.y,
                    };
                    // One wheel click is 60; a trackpad's pixels scale in step.
                    self.game
                        .wheel_zoom((amount / 60.0).clamp(-4.0, 4.0), self.game.cursor);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let key = input_key_name(event.logical_key, event.physical_key);
                if event.state == ElementState::Pressed {
                    self.keys.insert(key.clone());
                    if !event.repeat {
                        if key == "F11" {
                            self.game.action(game::Action::ToggleFullscreen);
                        } else if key == "F12" {
                            let path = self.game.data_dir.join("output/game-capture.png");
                            let _ =
                                std::fs::create_dir_all(path.parent().expect("capture has parent"));
                            match self.game.canvas.save(&path) {
                                Ok(()) => {
                                    self.game.notify("CAPTURE SAVED TO OUTPUT/GAME-CAPTURE.PNG")
                                }
                                Err(e) => self.game.notify(&e),
                            }
                        } else {
                            self.game.key(
                                &key,
                                self.modifiers.shift_key(),
                                self.modifiers.control_key() || self.modifiers.super_key(),
                            )
                        }
                    }
                } else {
                    self.keys.remove(&key);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if position.x >= 0.0
                    && position.y >= 0.0
                    && position.x < self.size.0 as f64
                    && position.y < self.size.1 as f64
                {
                    let pos = (position.x.floor() as i32, position.y.floor() as i32);
                    if let Some(last) = self.middle {
                        if self.game.screen == Screen::Match {
                            self.game.pan(last.0 - pos.0, last.1 - pos.1);
                            self.middle = Some(pos);
                        } else {
                            self.middle = None;
                        }
                    }
                    self.game.pointer_moved(pos.0, pos.1);
                } else {
                    self.game.pointer_left();
                    self.middle = None;
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let (x, y) = self.game.cursor;
                if !(0..self.size.0 as i32).contains(&x) || !(0..self.size.1 as i32).contains(&y) {
                    if state == ElementState::Released {
                        self.game.drag = None;
                        self.game.ux.minimap_drag = false;
                        self.middle = None;
                    }
                    return;
                }
                match (button, state) {
                    (MouseButton::Left, ElementState::Pressed) => {
                        self.game.ux.pointer_shift = self.modifiers.shift_key();
                        self.game.left_down(x, y);
                    }
                    (MouseButton::Left, ElementState::Released) => {
                        self.game.left_up(x, y, self.modifiers.shift_key())
                    }
                    (MouseButton::Right, ElementState::Pressed) => {
                        self.game.right_click(x, y, self.modifiers.shift_key())
                    }
                    (MouseButton::Middle, ElementState::Pressed)
                        if self.game.screen == Screen::Match
                            && self.game.world_pointer_allowed(x, y) =>
                    {
                        self.middle = Some((x, y));
                    }
                    (MouseButton::Middle, ElementState::Released) => self.middle = None,
                    _ => (),
                }
            }
            WindowEvent::RedrawRequested => self.draw(event_loop),
            _ => (),
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.size.0 == 0 || self.size.1 == 0 {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(100),
            ));
            self.last_time = Instant::now();
            return;
        }
        if Instant::now() >= self.frame_due {
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        } else {
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.frame_due));
        }
    }
}
fn input_key_name(logical: Key, physical: PhysicalKey) -> String {
    // Group bindings use the physical number row, including Shift+number
    // focus and Ctrl/Cmd+Shift+number reinforcement. Logical keys may be !/@/….
    let digit = match physical {
        PhysicalKey::Code(KeyCode::Digit0) => Some("0"),
        PhysicalKey::Code(KeyCode::Digit1) => Some("1"),
        PhysicalKey::Code(KeyCode::Digit2) => Some("2"),
        PhysicalKey::Code(KeyCode::Digit3) => Some("3"),
        PhysicalKey::Code(KeyCode::Digit4) => Some("4"),
        PhysicalKey::Code(KeyCode::Digit5) => Some("5"),
        PhysicalKey::Code(KeyCode::Digit6) => Some("6"),
        PhysicalKey::Code(KeyCode::Digit7) => Some("7"),
        PhysicalKey::Code(KeyCode::Digit8) => Some("8"),
        PhysicalKey::Code(KeyCode::Digit9) => Some("9"),
        _ => None,
    };
    digit
        .map(str::to_string)
        .unwrap_or_else(|| key_name(logical))
}
fn key_name(key: Key) -> String {
    match key {
        Key::Character(s) => s.to_uppercase(),
        Key::Named(n) => match n {
            NamedKey::Escape => "Escape",
            NamedKey::Enter => "Enter",
            NamedKey::Space => "Space",
            NamedKey::Home => "Home",
            NamedKey::Backspace => "Backspace",
            NamedKey::Tab => "Tab",
            NamedKey::ArrowLeft => "ArrowLeft",
            NamedKey::ArrowRight => "ArrowRight",
            NamedKey::ArrowUp => "ArrowUp",
            NamedKey::ArrowDown => "ArrowDown",
            NamedKey::F1 => "F1",
            NamedKey::F2 => "F2",
            NamedKey::F3 => "F3",
            NamedKey::F4 => "F4",
            NamedKey::F5 => "F5",
            NamedKey::F6 => "F6",
            NamedKey::F7 => "F7",
            NamedKey::F8 => "F8",
            NamedKey::F9 => "F9",
            NamedKey::F11 => "F11",
            NamedKey::F12 => "F12",
            _ => "",
        }
        .into(),
        _ => String::new(),
    }
}

/// The title bar and taskbar show the Union emblem, drawn twice size with
/// every texel kept square (the same art the release icon file carries).
fn window_icon(atlas: &canvas::Atlas) -> Option<winit::window::Icon> {
    let (rgba, w, h) = atlas.sprite_rgba("faction_union_emblem")?;
    let (bw, bh) = (w * 2, h * 2);
    let mut big = Vec::with_capacity((bw * bh * 4) as usize);
    for y in 0..bh {
        for x in 0..bw {
            let i = (((y / 2) * w + x / 2) * 4) as usize;
            big.extend_from_slice(&rgba[i..i + 4]);
        }
    }
    winit::window::Icon::from_rgba(big, bw, bh).ok()
}

fn project_root() -> PathBuf {
    if let Ok(path) = std::env::var("BRINEWAKE_ROOT") {
        return PathBuf::from(path);
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(contents) = exe.parent().and_then(|p| p.parent())
    {
        let resources = contents.join("Resources");
        if resources.join("art/exports/game-assets.png").exists() {
            return resources;
        }
    }
    let compiled = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    if compiled.join("art").exists() {
        return compiled;
    }
    if let Ok(exe) = std::env::current_exe() {
        for p in exe.ancestors() {
            if p.join("art/exports/game-assets.png").exists() {
                return p.to_path_buf();
            }
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
/// Edge-scroll direction for a cursor at the window's edges: (-1, 0, 1) per
/// axis.  The zone is `margin` pixels deep on every side of the whole window,
/// so hovering over the header or the dock still scrolls.  A cursor outside
/// the window (or a window with no size) never scrolls.
fn edge_scroll_delta(cursor: (i32, i32), size: (u32, u32), margin: i32) -> (i32, i32) {
    let (w, h) = (size.0 as i32, size.1 as i32);
    let (cx, cy) = cursor;
    if w <= 0 || h <= 0 || cx < 0 || cy < 0 || cx >= w || cy >= h {
        return (0, 0);
    }
    let margin = margin.max(1);
    let axis = |c: i32, extent: i32| {
        if c < margin {
            -1
        } else if c >= extent - margin {
            1
        } else {
            0
        }
    };
    (axis(cx, w), axis(cy, h))
}

/// `--resume RECORDING`: carry a lockstep match on from where the recording
/// ends. Every seat is given the same recording and plays it through, so the
/// match goes on from one position everywhere (an agent trial whose seats
/// had to be restarted).
fn resume_arg(arg: &dyn Fn(&str) -> Option<String>) -> Result<(), String> {
    let Some(path) = arg("--resume") else {
        return Ok(());
    };
    let mut player = bw_sim::ReplayPlayer::open(&path)?;
    // Every seat's summary and dock log are gathered on the way, so the
    // result screen covers the whole match and not only what follows the
    // restart.
    let history = match_summary::History::play(&mut player)?;
    let world = player.world().clone();
    if world.outcome.is_some() {
        return Err(format!("{path} is a finished match"));
    }
    println!("Resuming {path} at tick {}.", world.tick);
    net::session::resume_from(world, history)
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    let arg = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let base = project_root();
    if let Some(i) = args.iter().position(|a| a == "--trailer-scout") {
        let rest = &args[i + 1..];
        let seed = rest
            .first()
            .and_then(|s| s.parse().ok())
            .ok_or("--trailer-scout SEED MAP FACTION... [--minutes N]")?;
        let map = rest.get(1).ok_or("--trailer-scout SEED MAP FACTION...")?;
        let minutes = arg("--minutes").and_then(|m| m.parse().ok()).unwrap_or(20);
        let factions: Vec<String> = rest[2..]
            .iter()
            .take_while(|a| !a.starts_with("--"))
            .cloned()
            .collect();
        return trailer::scout(&base, seed, map, &factions, minutes);
    }
    if let Some(i) = args.iter().position(|a| a == "--trailer-audio") {
        let timeline = args
            .get(i + 1)
            .ok_or("--trailer-audio TIMELINE.json OUT.wav")?;
        let out = args
            .get(i + 2)
            .ok_or("--trailer-audio TIMELINE.json OUT.wav")?;
        return audio::render_trailer_audio(
            std::path::Path::new(timeline),
            std::path::Path::new(out),
        );
    }
    if let Some(i) = args.iter().position(|a| a == "--trailer-capture") {
        let shots = args
            .get(i + 1)
            .ok_or("--trailer-capture SHOTS.json DIR [NAME]")?;
        let dir = args
            .get(i + 2)
            .ok_or("--trailer-capture SHOTS.json DIR [NAME]")?;
        return trailer::export(
            &base,
            std::path::Path::new(shots),
            std::path::Path::new(dir),
            args.get(i + 3).map(String::as_str),
        );
    }
    if let Some(path) = arg("--compact-review") {
        return compact_review::export(base, std::path::Path::new(&path));
    }
    if let Some(path) = arg("--field-marks-review") {
        return field_marks_review::export(base, std::path::Path::new(&path));
    }
    if let Some(path) = arg("--readability-review") {
        let replay = arg("--replay").map(PathBuf::from);
        return readability_review::export(base, std::path::Path::new(&path), replay.as_deref());
    }
    if let Some(path) = arg("--qol-v3-review") {
        return qol_v3_review::export(base, std::path::Path::new(&path));
    }
    if let Some(path) = arg("--qol-v2-review") {
        return qol_v2_review::export(base, std::path::Path::new(&path));
    }
    if let Some(path) = arg("--art-v16-review") {
        return art_v16_review::export(base, std::path::Path::new(&path));
    }
    if let Some(path) = arg("--qol-review") {
        return qol::export(base, std::path::Path::new(&path));
    }
    if let Some(path) = arg("--art-v15-review") {
        art_v15_review::export(base, std::path::Path::new(&path))?;
        println!("Controlled v15 art captures saved to {path}");
        return Ok(());
    }
    if let Some(path) = arg("--native-review") {
        native_review::export(base, std::path::Path::new(&path))?;
        return Ok(());
    }
    if let Some(path) = arg("--zoom-review") {
        camera_review::export_zoom(base, std::path::Path::new(&path))?;
        return Ok(());
    }
    if let Some(path) = arg("--camera-review") {
        camera_review::export(base, std::path::Path::new(&path))?;
        return Ok(());
    }
    if let Some(path) = arg("--chart-review") {
        chart_review::export(base, std::path::Path::new(&path))?;
        return Ok(());
    }
    if let Some(path) = arg("--polish-journeys") {
        polish_journeys::export(base, std::path::Path::new(&path))?;
        return Ok(());
    }
    if let Some(path) = arg("--render-review") {
        render_review::export(base, std::path::Path::new(&path))?;
        return Ok(());
    }
    if let Some(path) = arg("--audio-review") {
        audio::export_review(std::path::Path::new(&path))?;
        return Ok(());
    }
    if let Some(path) = arg("--music-timeline") {
        music_timeline::export(std::path::Path::new(&path))?;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--sound-replay") {
        let paths: Vec<String> = std::env::args()
            .skip_while(|a| a != "--sound-replay")
            .skip(1)
            .collect();
        print!("{}", sound_director::measure_replays(&paths)?);
        return Ok(());
    }
    if let Some(path) = arg("--soundtrack-review") {
        audio::export_soundtrack(std::path::Path::new(&path))?;
        println!("Soundtrack and effects review saved to {path}");
        return Ok(());
    }
    if let Some(path) = arg("--depth-review") {
        depth_review::export(base, &PathBuf::from(&path))?;
        println!("Controlled depth UI fixtures saved to {path}");
        return Ok(());
    }
    if let Some(path) = arg("--ux-review") {
        ux_review::export(base, &PathBuf::from(&path))?;
        println!("Controlled UI journey and screenshots saved to {path}");
        return Ok(());
    }
    let mut game = if let Some(path) = arg("--data-dir") {
        Game::new_with_data_dir(base.clone(), PathBuf::from(path))
    } else {
        Game::new(base.clone())
    };
    if args.iter().any(|a| a == "--assembly") {
        game.faction = bw_core::Faction::Assembly;
    }
    // The side this seat plays when it hosts or starts a skirmish.
    if let Some(name) = arg("--faction") {
        game.faction = match name.as_str() {
            "union" => bw_core::Faction::Union,
            "assembly" => bw_core::Faction::Assembly,
            "compact" => bw_core::Faction::Compact,
            other => {
                return Err(format!(
                    "unknown faction {other}: use union, assembly or compact"
                ));
            }
        };
    }
    // The map a skirmish is played on: the Split Basin unless named.
    if let Some(name) = arg("--map") {
        game.map = match name.as_str() {
            "confluence" => bw_sim::MapId::Confluence,
            "split-basin" | "basin" => bw_sim::MapId::SplitBasin,
            other => {
                return Err(format!(
                    "unknown map {other}: use split-basin or confluence"
                ));
            }
        };
    }
    let seed = arg("--seed")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1);
    // A fixed input delay in ticks; without it the delay follows the
    // connection.
    let delay = arg("--delay").and_then(|s| s.parse::<u64>().ok());
    if let Some(text) = arg("--net-sim") {
        // A deliberately bad network for this seat's outgoing packets.
        net::Conditions::parse(&text)?.choose();
    }
    if let (Some(out), Some(replay)) = (arg("--spectate-review"), arg("--replay")) {
        spectator::export_review(
            &mut game,
            std::path::Path::new(&replay),
            &PathBuf::from(&out),
        )?;
        println!("Spectator and result captures saved to {out}");
        return Ok(());
    }
    if let Some(path) = arg("--replay") {
        let follow = args.iter().any(|a| a == "--follow");
        game.start_playback(std::path::Path::new(&path), follow)?;
        println!(
            "Observing {path}{}.",
            if follow {
                " and following it as it grows"
            } else {
                ""
            }
        );
    } else if let Some(addr) = arg("--host") {
        resume_arg(&arg)?;
        println!("Hosting a lockstep match on {addr}; waiting for the other players.");
        let session = net::lobby::host_blocking(
            addr.as_str(),
            seed,
            game.faction,
            game.map,
            delay,
            net::lobby::ACCEPT_WAIT,
        )?;
        game.start_network(session);
    } else if let Some(target) = arg("--join") {
        resume_arg(&arg)?;
        println!("Joining the lockstep match at {target}.");
        let session = net::lobby::join_blocking(
            target.as_str(),
            Duration::from_secs(60),
            args.iter()
                .any(|a| a == "--faction")
                .then_some(game.faction),
        )?;
        game.start_network(session);
    } else if args.iter().any(|a| a == "--play") {
        game.start();
    }
    if args.iter().any(|a| a == "--qol-v2-practice") {
        qol_v2_review::prepare(&mut game)?;
    }
    if args.iter().any(|a| a == "--depth-practice") {
        depth_review::practice(&mut game)?;
    }
    if args.iter().any(|a| a == "--artistry-practice") {
        artistry_review::practice(&mut game)?;
    }
    if let Some(path) = arg("--artistry-review") {
        artistry_review::export(&mut game, &PathBuf::from(&path))?;
        return Ok(());
    }
    if let Some(path) = arg("--art-review") {
        art_review::export(&mut game, &PathBuf::from(&path))?;
        println!(
            "Offscreen art-state fixtures saved to {path}; these are not live gameplay captures."
        );
        return Ok(());
    }
    if let Some(path) = arg("--screenshot") {
        let path = PathBuf::from(path);
        game.screenshot(&path)?;
        println!("Offscreen 640x360 composition saved to {}", path.display());
        return Ok(());
    }
    let size = arg("--size")
        .and_then(|s| {
            s.split_once('x')
                .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
        })
        .unwrap_or((1920, 1080));
    let quit_after = arg("--smoke-seconds")
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs);
    let capture = arg("--capture").map(PathBuf::from);
    let agent_port = arg("--agent-port").and_then(|s| s.parse::<u16>().ok());
    if args.iter().any(|a| a == "--headless") {
        return agent::run_headless(game, size, agent_port, quit_after);
    }
    if agent_port.is_some() {
        return Err("--agent-port needs --headless".to_string());
    }
    let mut event_loop_builder = EventLoop::builder();
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::EventLoopBuilderExtMacOS;
        // Winit's default Quit menu intercepts Cmd-Q and terminates immediately.
        // Let our field-aware keyboard handler offer Save / Keep / Quit instead.
        event_loop_builder.with_default_menu(false);
    }
    let event_loop = event_loop_builder.build().map_err(|e| e.to_string())?;
    let mut app = App::new(game, size, quit_after, capture);
    event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
    if let Some(error) = app.error {
        Err(error)
    } else {
        Ok(())
    }
}
fn main() {
    if let Err(e) = run() {
        eprintln!("Unable to run BRINEWAKE: {e}");
        show_error(&format!("Unable to run BRINEWAKE: {e}"));
        std::process::exit(1)
    }
}

/// Without a console, a Windows player would see nothing at all when the
/// game cannot start (no graphics card it can use, art missing): say why.
#[cfg(all(windows, not(debug_assertions)))]
fn show_error(message: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (text, title) = (wide(message), wide("BRINEWAKE"));
    // SAFETY: both strings are NUL-terminated and outlive the call.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
#[cfg(not(all(windows, not(debug_assertions))))]
fn show_error(_message: &str) {}

#[cfg(test)]
mod input_recovery_tests {
    #[test]
    fn edge_scroll_zones_cover_the_whole_window_including_header_and_dock() {
        let size = (1920, 1080);
        // Top edge over the header and bottom edge over the dock both scroll.
        assert_eq!(super::edge_scroll_delta((960, 2), size, 8), (0, -1));
        assert_eq!(super::edge_scroll_delta((960, 1079), size, 8), (0, 1));
        // Corners scroll on both axes; sides scroll on one.
        assert_eq!(super::edge_scroll_delta((3, 5), size, 8), (-1, -1));
        assert_eq!(super::edge_scroll_delta((1915, 500), size, 8), (1, 0));
        // The world band's own edges, just above the dock, no longer scroll.
        assert_eq!(super::edge_scroll_delta((960, 838), size, 8), (0, 0));
        assert_eq!(super::edge_scroll_delta((960, 66), size, 8), (0, 0));
        // Outside the window (the pointer left) and a zero-sized window: never.
        assert_eq!(super::edge_scroll_delta((-999, -999), size, 8), (0, 0));
        assert_eq!(super::edge_scroll_delta((1920, 500), size, 8), (0, 0));
        assert_eq!(super::edge_scroll_delta((5, 5), (0, 0), 8), (0, 0));
    }

    use super::*;

    #[test]
    fn victory_inside_a_catchup_burst_starts_aftermath_on_its_own_clock() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        for speed in tempo::GameSpeed::all() {
            let mut g = Game::new_with_data_dir(
                base.clone(),
                std::env::temp_dir().join(format!(
                    "bw-qol2-terminal-{}-{}",
                    std::process::id(),
                    speed.label()
                )),
            );
            g.begin_practice();
            g.intro = None;
            g.ux.preferences.game_speed = speed;
            g.world.issue(0, bw_sim::Command::Surrender).unwrap();
            let mut app = App::new(g, (960, 540), None, None);
            app.advance_clocks(0.2 / speed.multiplier());
            assert!(app.game.world.outcome.is_some());
            assert_eq!(
                app.game.aftermath_ticks, 0,
                "old field budget must not advance aftermath"
            );
            let tick = app.game.world.tick;
            app.advance_clocks(1.0 / 30.0);
            assert_eq!(app.game.aftermath_ticks, 1);
            assert_eq!(app.game.world.tick, tick);
        }
    }

    #[test]
    fn actual_app_pace_changes_only_wall_time_and_resets_on_pause_and_speed_changes() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut hashes = Vec::new();
        for speed in tempo::GameSpeed::all() {
            let mut g = Game::new_with_data_dir(
                base.clone(),
                std::env::temp_dir().join(format!(
                    "bw-qol2-clock-{}-{}",
                    std::process::id(),
                    speed.label()
                )),
            );
            g.begin_practice();
            g.intro = None;
            g.ux.preferences.game_speed = speed;
            let mut app = App::new(g, (960, 540), None, None);
            for _ in 0..120 {
                app.advance_clocks((1.0 / 30.0) / speed.multiplier());
            }
            assert_eq!(app.game.world.tick, 120);
            hashes.push(app.game.world.state_hash());
            app.game.key("Escape", false, false);
            app.advance_clocks(60.0);
            app.advance_clocks(60.0);
            assert_eq!(app.game.world.tick, 120);
            app.game.key("Escape", false, false);
            app.advance_clocks(60.0);
            assert_eq!(
                app.game.world.tick, 120,
                "resume must discard elapsed menu time"
            );
            app.game.ux.preferences.game_speed = speed.cycle();
            app.advance_clocks(60.0);
            assert_eq!(
                app.game.world.tick, 120,
                "speed changes must discard backlog"
            );
            app.game.ux.practice_review = true;
            app.advance_clocks(0.0);
            for _ in 0..30 {
                app.advance_clocks(1.0 / 30.0);
            }
            assert_eq!(
                app.game.aftermath_ticks, 30,
                "aftermath stays at normal pace"
            );
        }
        assert!(hashes.windows(2).all(|w| w[0] == w[1]));
        assert_eq!(key_name(Key::Named(NamedKey::F3)), "F3");
        assert_eq!(key_name(Key::Named(NamedKey::F4)), "F4");
    }

    #[test]
    fn f7_from_the_keyboard_selects_every_worker() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut game = Game::new_with_data_dir(
            base,
            std::env::temp_dir().join(format!("bw-f7-key-{}", std::process::id())),
        );
        game.begin_practice();
        game.intro = None;
        let mut workers: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .collect();
        workers.sort_unstable();
        assert!(!workers.is_empty());
        // The name winit's F7 press becomes is what the game binds.
        let key = input_key_name(Key::Named(NamedKey::F7), PhysicalKey::Code(KeyCode::F7));
        game.key(&key, false, false);
        assert_eq!(game.selected, workers);
    }

    #[test]
    fn app_switch_clears_held_input_and_timing_without_automatic_resume() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut game = Game::new_with_data_dir(
            base,
            std::env::temp_dir().join(format!("bw-qol-app-{}", std::process::id())),
        );
        game.begin_practice();
        game.intro = None;
        let mut app = App::new(game, (1920, 1080), None, None);
        app.keys.insert("ArrowRight".into());
        app.modifiers = ModifiersState::SHIFT;
        app.middle = Some((50, 50));
        app.accumulator = 5.0;
        app.set_focus(false);
        assert!(app.keys.is_empty() && app.modifiers.is_empty() && app.middle.is_none());
        assert_eq!(app.accumulator, 0.0);
        assert_eq!(app.game.screen, Screen::Pause);
        app.accumulator = 60.0;
        app.set_focus(true);
        assert_eq!(app.accumulator, 0.0);
        assert_eq!(app.game.screen, Screen::Pause);
        assert_eq!(key_name(Key::Named(NamedKey::F2)), "F2");
        for (key, name) in [
            (NamedKey::F1, "F1"),
            (NamedKey::F2, "F2"),
            (NamedKey::F3, "F3"),
            (NamedKey::F4, "F4"),
            (NamedKey::F5, "F5"),
            (NamedKey::F6, "F6"),
            (NamedKey::F7, "F7"),
            (NamedKey::F8, "F8"),
            (NamedKey::F9, "F9"),
            (NamedKey::F11, "F11"),
            (NamedKey::F12, "F12"),
        ] {
            assert_eq!(
                key_name(Key::Named(key)),
                name,
                "a bound function key needs a name"
            );
        }
        assert_eq!(
            input_key_name(
                Key::Character("!".into()),
                PhysicalKey::Code(KeyCode::Digit1)
            ),
            "1"
        );
        assert_eq!(
            input_key_name(
                Key::Character(")".into()),
                PhysicalKey::Code(KeyCode::Digit0)
            ),
            "0"
        );
    }
}
