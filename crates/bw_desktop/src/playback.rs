//! Observer playback: a recorded match replayed tick by tick with the fog
//! lifted, at the player's pace, with pause but without orders.
//!
//! With `follow` the recording may still be growing: when playback reaches
//! its end the file is re-read every second and any new commands are taken
//! up, so a live match can be watched a few seconds behind its host.
//!
//! A finished recording is also scanned once on a background thread: the
//! scan fills the whole match's summary for the replay timeline and keeps a
//! world every fifteen seconds of match time, so a click on the timeline
//! seeks by stepping at most fifteen seconds from the nearest one.

use crate::dock_log::DockState;
use crate::match_summary::MatchSummary;
use bw_sim::{ReplayPlayer, World};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const RELOAD_EVERY: Duration = Duration::from_secs(1);
/// Ticks behind the recording's end before a followed playback speeds up.
const CATCH_UP_NEAR: u64 = 5 * 30;
const CATCH_UP_FAR: u64 = 20 * 30;
/// Match time between the scan's kept worlds.
pub const SNAPSHOT_EVERY: u64 = 450;
/// How often the scan publishes its summary so far.
const PUBLISH_EVERY: u64 = 900;

/// What the background scan knows so far.
#[derive(Default)]
pub struct ReplayIndex {
    pub summary: MatchSummary,
    /// Worlds at every `SNAPSHOT_EVERY` ticks, the first at the start.
    snapshots: Vec<World>,
    /// The dock log as it stood at each kept world, so a seek shows the
    /// match's history up to where it lands.
    docks: Vec<DockState>,
    /// The tick the scan has reached.
    pub scanned: u64,
    pub done: bool,
    pub error: Option<String>,
}

pub struct Playback {
    player: ReplayPlayer,
    path: PathBuf,
    pub follow: bool,
    last_reload: Instant,
    /// The last error from re-reading the file, shown once.
    pub notice: Option<String>,
    index: Option<Arc<Mutex<ReplayIndex>>>,
    cancel: Arc<AtomicBool>,
}

impl Drop for Playback {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// Step a copy of the recording to its end, sampling the summary and keeping
/// worlds to seek from.
fn scan(path: PathBuf, index: Arc<Mutex<ReplayIndex>>, cancel: Arc<AtomicBool>) {
    let mut player = match ReplayPlayer::open(&path) {
        Ok(player) => player,
        Err(e) => {
            if let Ok(mut index) = index.lock() {
                index.error = Some(e);
                index.done = true;
            }
            return;
        }
    };
    let mut summary = MatchSummary::default();
    let mut dock = DockState::default();
    dock.reset_for_new_match();
    let mut pending = vec![player.world().clone()];
    let mut pending_docks = vec![dock.clone()];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        let stepped = match player.step() {
            Ok(stepped) => stepped,
            Err(e) => {
                if let Ok(mut index) = index.lock() {
                    index.error = Some(e);
                }
                false
            }
        };
        let tick = player.tick();
        if stepped {
            summary.observe(player.world());
            dock.after_authoritative_tick(player.world());
            if tick.is_multiple_of(SNAPSHOT_EVERY) {
                pending.push(player.world().clone());
                pending_docks.push(dock.clone());
            }
        }
        if !stepped || tick.is_multiple_of(PUBLISH_EVERY) {
            let Ok(mut index) = index.lock() else { return };
            index.snapshots.append(&mut pending);
            index.docks.append(&mut pending_docks);
            index.summary = summary.clone();
            index.scanned = tick;
            if !stepped {
                index.done = true;
                return;
            }
        }
    }
}

impl Playback {
    pub fn open(path: &Path, follow: bool) -> Result<Self, String> {
        let player = ReplayPlayer::open(path)?;
        let cancel = Arc::new(AtomicBool::new(false));
        // A growing file has no end to scan to: its timeline is the one the
        // observer has watched so far.
        let index = (!follow).then(|| {
            let index = Arc::new(Mutex::new(ReplayIndex::default()));
            let (path, shared, cancel) = (path.to_path_buf(), index.clone(), cancel.clone());
            std::thread::spawn(move || scan(path, shared, cancel));
            index
        });
        Ok(Self {
            player,
            path: path.to_path_buf(),
            follow,
            last_reload: Instant::now(),
            notice: None,
            index,
            cancel,
        })
    }

    /// The observed world with the fog lifted.
    pub fn view(&self) -> World {
        let mut world = self.player.world().clone();
        world.revealed = true;
        world
    }

    pub fn tick(&self) -> u64 {
        self.player.tick()
    }

    pub fn end_tick(&self) -> u64 {
        self.player.end_tick()
    }

    /// How many ticks to play for each one due: a followed recording that
    /// has fallen behind the live match plays at 2X, far behind at 4X, so
    /// the observer never stays half a minute late.
    pub fn catch_up_factor(&self) -> u32 {
        if !self.follow {
            return 1;
        }
        match self.end_tick().saturating_sub(self.tick()) {
            behind if behind > CATCH_UP_FAR => 4,
            behind if behind > CATCH_UP_NEAR => 2,
            _ => 1,
        }
    }

    /// The whole match's summary once the scan has finished, or as far as it
    /// has got, with the tick it reached and whether it is done.
    pub fn index_summary(&self) -> Option<(MatchSummary, u64, bool)> {
        let index = self.index.as_ref()?.lock().ok()?;
        (index.scanned > 0 || index.done)
            .then(|| (index.summary.clone(), index.scanned, index.done))
    }

    /// True when the timeline can seek: a finished recording.
    pub fn can_seek(&self) -> bool {
        self.index.is_some()
    }

    /// Move playback to `tick`, from the nearest kept world at or before it.
    /// A tick the scan has not reached yet is clamped to where it has got.
    /// `dock` follows: it restarts from the kept world's log and records
    /// every tick stepped over, so the log is whole where playback lands.
    pub fn seek(&mut self, tick: u64, dock: &mut DockState) -> Result<u64, String> {
        let Some(index) = &self.index else {
            return Err("A live recording cannot seek.".into());
        };
        let (start, reached) = {
            let index = index
                .lock()
                .map_err(|_| "The replay scan stopped.".to_string())?;
            let target = tick.min(index.scanned.max(self.player.tick()));
            let start = index
                .snapshots
                .iter()
                .zip(index.docks.iter())
                .rev()
                .find(|(world, _)| world.tick <= target)
                .map(|(world, dock)| (world.clone(), dock.clone()));
            (start, target)
        };
        if let Some((start, kept)) = start
            && (reached < self.player.tick() || reached - self.player.tick() > SNAPSHOT_EVERY)
        {
            self.player.restore(start);
            *dock = kept;
        } else if reached < self.player.tick() {
            return Err("The replay scan has not started yet.".into());
        }
        while self.player.tick() < reached {
            if !self.player.step()? {
                break;
            }
            dock.after_authoritative_tick(self.player.world());
        }
        Ok(self.player.tick())
    }

    /// Advance one tick.  At the end of a followed file, re-read it at most
    /// once a second; returns false while there is nothing new to play.
    pub fn step(&mut self) -> bool {
        if self.player.finished() {
            if !self.follow || self.last_reload.elapsed() < RELOAD_EVERY {
                return false;
            }
            self.last_reload = Instant::now();
            match self.player.extend_from(&self.path) {
                Ok(true) => {}
                Ok(false) => return false,
                Err(e) => {
                    self.notice = Some(e);
                    return false;
                }
            }
        }
        match self.player.step() {
            Ok(stepped) => stepped,
            Err(e) => {
                self.notice = Some(e);
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bw_core::{Faction, Pos};

    #[test]
    fn playback_lifts_the_fog_and_follows_a_growing_recording() {
        let dir = std::env::temp_dir().join(format!("brinewake-playback-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("live.replay.json");
        let mut world = World::new(29, Faction::Assembly);
        world.ai_enabled = false;
        for _ in 0..30 {
            world.step();
        }
        world.export_replay(&path).unwrap();
        let mut playback = Playback::open(&path, true).unwrap();
        assert!(!playback.view().visible(0, Pos::cell(120, 120)) || playback.view().revealed);
        while playback.step() {}
        assert_eq!(playback.tick(), 30);
        let view = playback.view();
        assert!(view.revealed);
        assert!(view.visible(0, Pos::cell(120, 120)));
        assert!(view.visible(0, Pos::cell(3, 3)));
        for _ in 0..30 {
            world.step();
        }
        world.export_replay(&path).unwrap();
        playback.last_reload = Instant::now() - RELOAD_EVERY;
        assert!(playback.step(), "the grown file is taken up");
        while playback.step() {}
        assert_eq!(playback.tick(), 60);
        assert_eq!(playback.view().state_hash(), world.state_hash());
        assert_eq!(playback.catch_up_factor(), 1, "caught up");
        for _ in 0..CATCH_UP_FAR + 30 {
            world.step();
        }
        world.export_replay(&path).unwrap();
        playback.last_reload = Instant::now() - RELOAD_EVERY;
        assert!(playback.step());
        assert_eq!(playback.catch_up_factor(), 4, "far behind a live match");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_finished_recording_is_scanned_and_seeks_both_ways() {
        let dir = std::env::temp_dir().join(format!("brinewake-seek-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("done.replay.json");
        let mut world = World::new(31, Faction::Union);
        for _ in 0..(SNAPSHOT_EVERY * 3 + 100) {
            world.step();
        }
        world.export_replay(&path).unwrap();
        // The recording played straight through is the reference: a replay
        // carries the computer's orders as commands, not its planner.
        let mut straight = Playback::open(&path, true).unwrap();
        let mut hashes = std::collections::BTreeMap::new();
        while straight.step() {
            if straight.tick().is_multiple_of(200) {
                hashes.insert(straight.tick(), straight.view().state_hash());
            }
        }
        let final_hash = straight.view().state_hash();
        let mut playback = Playback::open(&path, false).unwrap();
        let started = Instant::now();
        while !playback.index_summary().is_some_and(|(_, _, done)| done) {
            assert!(
                started.elapsed() < Duration::from_secs(120),
                "scan finishes"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        let (summary, scanned, _) = playback.index_summary().unwrap();
        assert_eq!(scanned, world.tick);
        assert_eq!(
            summary.samples.len() as u64,
            world.tick / crate::match_summary::SAMPLE_EVERY
        );
        for target in [1000, 200, 1200, 400] {
            assert_eq!(
                playback.seek(target, &mut DockState::default()).unwrap(),
                target
            );
            assert_eq!(
                playback.view().state_hash(),
                hashes[&target],
                "seek to {target}"
            );
        }
        while playback.step() {}
        assert_eq!(playback.view().state_hash(), final_hash);
        let _ = std::fs::remove_dir_all(dir);
    }
}
