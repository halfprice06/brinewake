//! Wall time per system inside `World::step`, for a tick-cost watchdog.
//!
//! Off unless a caller turns it on for its thread, and then it only reads
//! the clock between systems: it never touches the world, so a profiled
//! step is the same step. Trial 10 crawled at four seconds a tick for ten
//! minutes before anyone could say which system was to blame.

use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};

thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    static TOTALS: RefCell<Vec<(&'static str, Duration)>> = const { RefCell::new(Vec::new()) };
}

/// Turn per-system timing on or off for steps run on this thread.
pub fn enable(on: bool) {
    ENABLED.with(|e| e.set(on));
    if !on {
        TOTALS.with(|t| t.borrow_mut().clear());
    }
}

/// Whether steps on this thread are timed.
pub fn enabled() -> bool {
    ENABLED.with(Cell::get)
}

/// Each system's time over the steps since the last call, in step order,
/// and start again from nothing.
pub fn take() -> Vec<(&'static str, Duration)> {
    TOTALS.with(|t| std::mem::take(&mut *t.borrow_mut()))
}

/// The system that took longest in `laps`, if any.
pub fn hottest(laps: &[(&'static str, Duration)]) -> Option<(&'static str, Duration)> {
    laps.iter().copied().max_by_key(|(_, d)| *d)
}

/// Times the systems of one step: `lap` after each names it.
pub(crate) struct Probe {
    last: Option<Instant>,
}

impl Probe {
    pub(crate) fn start() -> Probe {
        Probe {
            last: enabled().then(Instant::now),
        }
    }

    pub(crate) fn lap(&mut self, system: &'static str) {
        let Some(last) = self.last else { return };
        let now = Instant::now();
        let spent = now.duration_since(last);
        self.last = Some(now);
        TOTALS.with(|t| {
            let mut totals = t.borrow_mut();
            match totals.iter_mut().find(|(name, _)| *name == system) {
                Some((_, total)) => *total += spent,
                None => totals.push((system, spent)),
            }
        });
    }
}

/// Run one system of a step and note its time on `probe`.
macro_rules! system {
    ($probe:ident, $self:ident . $name:ident ( $($arg:expr),* )) => {{
        let out = $self.$name($($arg),*);
        $probe.lap(stringify!($name));
        out
    }};
}
pub(crate) use system;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::World;
    use bw_core::Faction;

    #[test]
    fn a_profiled_step_is_the_same_step_and_names_every_system() {
        let mut plain = World::new(7, Faction::Union);
        let mut timed = World::new(7, Faction::Union);
        enable(false);
        for _ in 0..40 {
            plain.step();
        }
        assert!(take().is_empty(), "nothing is timed while off");
        enable(true);
        for _ in 0..40 {
            timed.step();
        }
        let laps = take();
        enable(false);
        assert_eq!(plain.state_hash(), timed.state_hash());
        assert_eq!(plain, timed);
        let names: Vec<&str> = laps.iter().map(|(n, _)| *n).collect();
        for system in [
            "run_ai",
            "update_navigation",
            "update_combat",
            "update_victory",
        ] {
            assert!(names.contains(&system), "{system} in {names:?}");
        }
        assert_eq!(names.first(), Some(&"apply_gate_schedule"), "step order");
        assert!(hottest(&laps).is_some());
        assert!(take().is_empty(), "take starts again from nothing");
    }
}
