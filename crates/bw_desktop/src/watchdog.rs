//! Tick-cost watchdog: a line on stderr for any game tick over a limit,
//! naming the simulation system that took longest.
//!
//! Trial 10 crawled at about four seconds a tick for ten minutes (an attack
//! order at a target across water that had just gone deep ran every failing
//! path search again each tick); it took gdb to say where. This says so the
//! first time a tick runs long.
//!
//! On in debug builds at 250 ms. A release build is unchanged unless
//! `BRINEWAKE_WATCHDOG_MS` is set (the agent harness sets it): a number of
//! milliseconds turns it on at that limit, `0` or `off` turns it off. It
//! only reads the clock and writes to stderr; it never changes the match.

use crate::game::Game;
use std::time::{Duration, Instant};

/// The limit a debug build starts with.
pub(crate) const DEFAULT_LIMIT: Duration = Duration::from_millis(250);
/// Slow ticks within this of a report are counted into the next one.
const REPORT_EVERY: Duration = Duration::from_secs(1);

#[derive(Debug, Default)]
pub(crate) struct Watchdog {
    limit: Option<Duration>,
    last_report: Option<Instant>,
    /// Slow ticks since the last report that it did not name.
    held: u32,
    /// Every tick over the limit, the slowest, and the last report.
    pub(crate) slow_ticks: u64,
    pub(crate) worst: Duration,
    pub(crate) last: Option<String>,
}

/// The limit from the environment variable's value, and the build.
fn limit_from(var: Option<&str>, debug: bool) -> Option<Duration> {
    match var.map(str::trim) {
        Some("0" | "off" | "OFF") => None,
        Some(text) => match text.parse::<u64>() {
            Ok(ms) => Some(Duration::from_millis(ms)),
            Err(_) => debug.then_some(DEFAULT_LIMIT),
        },
        None => debug.then_some(DEFAULT_LIMIT),
    }
}

fn ms(d: Duration) -> u128 {
    d.as_millis()
}

impl Watchdog {
    pub(crate) fn from_env() -> Watchdog {
        let var = std::env::var("BRINEWAKE_WATCHDOG_MS").ok();
        Watchdog::with_limit(limit_from(var.as_deref(), cfg!(debug_assertions)))
    }

    pub(crate) fn with_limit(limit: Option<Duration>) -> Watchdog {
        Watchdog {
            limit,
            ..Default::default()
        }
    }

    /// Run one game tick, timed when the watchdog is on.
    pub(crate) fn tick(&mut self, game: &mut Game) {
        if self.limit.is_none() {
            game.tick();
            return;
        }
        bw_sim::profile::enable(true);
        let before = game.world.tick;
        let started = Instant::now();
        game.tick();
        let spent = started.elapsed();
        let laps = bw_sim::profile::take();
        if let Some(line) = self.observe(before, spent, &laps, Instant::now()) {
            eprintln!("{line}");
        }
    }

    /// Note one tick's cost; the line to report, if it is one.
    pub(crate) fn observe(
        &mut self,
        tick: u64,
        spent: Duration,
        laps: &[(&'static str, Duration)],
        now: Instant,
    ) -> Option<String> {
        let limit = self.limit?;
        if spent < limit {
            return None;
        }
        self.slow_ticks += 1;
        self.worst = self.worst.max(spent);
        let simulation: Duration = laps.iter().map(|(_, d)| *d).sum();
        let mut line = format!("WATCHDOG: tick {tick} took {} ms", ms(spent));
        match bw_sim::profile::hottest(laps) {
            Some((system, hot)) => {
                line.push_str(&format!(
                    " (simulation {} ms, hottest {system} {} ms)",
                    ms(simulation),
                    ms(hot)
                ));
            }
            _ => line.push_str(" (outside the simulation)"),
        }
        if self
            .last_report
            .is_some_and(|at| now.duration_since(at) < REPORT_EVERY)
        {
            self.held += 1;
            self.last = Some(line);
            return None;
        }
        if self.held > 0 {
            line.push_str(&format!(
                "; {} more slow ticks since the last line",
                self.held
            ));
            self.held = 0;
        }
        self.last_report = Some(now);
        self.last = Some(line.clone());
        Some(line)
    }

    /// For the agent harness's `/state`: null while the watchdog is off.
    pub(crate) fn json(&self) -> serde_json::Value {
        match self.limit {
            None => serde_json::Value::Null,
            Some(limit) => serde_json::json!({
                "limit_ms": ms(limit) as u64,
                "slow_ticks": self.slow_ticks,
                "worst_ms": ms(self.worst) as u64,
                "last": self.last,
            }),
        }
    }
}

/// How long the headless loop runs ticks before it answers the agent port
/// in between: a crawling match still answers within about one tick.
pub(crate) const BURST_BUDGET: Duration = Duration::from_millis(250);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_limit_is_on_in_debug_and_off_in_release_unless_asked() {
        assert_eq!(limit_from(None, true), Some(DEFAULT_LIMIT));
        assert_eq!(limit_from(None, false), None, "release is unchanged");
        assert_eq!(
            limit_from(Some("400"), false),
            Some(Duration::from_millis(400))
        );
        assert_eq!(limit_from(Some("0"), true), None);
        assert_eq!(limit_from(Some("off"), true), None);
        assert_eq!(limit_from(Some("soon"), false), None);
    }

    #[test]
    fn a_slow_tick_names_its_hottest_system_and_reports_are_rate_limited() {
        let mut dog = Watchdog::with_limit(Some(Duration::from_millis(250)));
        let t0 = Instant::now();
        let laps = [
            ("run_ai", Duration::from_millis(3)),
            ("update_navigation", Duration::from_millis(6690)),
            ("update_combat", Duration::from_millis(7)),
        ];
        assert_eq!(dog.observe(10, Duration::from_millis(40), &laps, t0), None);
        assert_eq!(dog.slow_ticks, 0);
        let line = dog
            .observe(39050, Duration::from_millis(6712), &laps, t0)
            .expect("a slow tick is reported");
        assert!(line.contains("tick 39050 took 6712 ms"), "{line}");
        assert!(line.contains("hottest update_navigation 6690 ms"), "{line}");
        assert!(line.contains("simulation 6700 ms"), "{line}");
        // Another at once is counted, not printed; the next line says so.
        let soon = t0 + Duration::from_millis(300);
        assert_eq!(
            dog.observe(39051, Duration::from_secs(5), &laps, soon),
            None
        );
        let later = t0 + Duration::from_secs(2);
        let line = dog
            .observe(39052, Duration::from_millis(300), &[], later)
            .expect("reported after the quiet second");
        assert!(line.contains("outside the simulation"), "{line}");
        assert!(line.contains("1 more slow ticks"), "{line}");
        assert_eq!(dog.slow_ticks, 3);
        assert_eq!(dog.worst, Duration::from_millis(6712));
        assert_eq!(dog.json()["slow_ticks"], 3);
    }

    #[test]
    fn an_off_watchdog_reports_nothing() {
        let mut dog = Watchdog::with_limit(None);
        let line = dog.observe(1, Duration::from_secs(9), &[], Instant::now());
        assert_eq!(line, None);
        assert!(dog.json().is_null());
    }
}
