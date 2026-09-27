//! Wall-clock pacing for the fixed-rate desktop simulation.
//!
//! `GameSpeed` changes how quickly wall time becomes due simulation time.  It
//! never changes the authoritative step itself: every returned step is still
//! one `1 / 30` second simulation tick.  Presentation clocks that should stay
//! responsive (the field guide and post-match aftermath) can pass
//! [`GameSpeed::Normal`] explicitly.

use serde::de::Deserializer;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The authoritative simulation rate.  This is shared with the desktop
/// scheduler's existing `1.0 / 30.0` step.
pub const TICKS_PER_SECOND: u32 = 30;
pub const SIMULATION_TICK_SECONDS: f64 = 1.0 / TICKS_PER_SECOND as f64;

/// Maximum fixed steps that one render frame may release.  Pending wall time
/// above this bound is intentionally discarded to avoid an unbounded offline
/// catch-up burst after a pause, focus change, or long scheduler stall.
pub const MAX_CATCH_UP_TICKS: u32 = 6;
const MAX_BACKLOG_SECONDS: f64 = SIMULATION_TICK_SECONDS * MAX_CATCH_UP_TICKS as f64;

/// User-selectable simulation pacing.
///
/// Serialization keeps the stable variant names (`"Slow"`, `"Normal"`, and
/// `"Fast"`).  Deserialization also accepts the displayed multiplier labels
/// and numeric multiplier values, while unknown or malformed saved values
/// safely fall back to [`GameSpeed::Normal`].  This keeps an unsupported
/// future preference from making the whole preference sidecar unreadable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum GameSpeed {
    Slow,
    #[default]
    Normal,
    Fast,
}

impl GameSpeed {
    /// The compact label intended for the speed button and Settings row.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Slow => "0.5X",
            Self::Normal => "1X",
            Self::Fast => "1.5X",
        }
    }

    /// Wall time is multiplied by this value before it enters the fixed-step
    /// accumulator.  The simulation remains at [`SIMULATION_TICK_SECONDS`].
    pub const fn multiplier(self) -> f64 {
        match self {
            Self::Slow => 0.5,
            Self::Normal => 1.0,
            Self::Fast => 1.5,
        }
    }

    /// Advance the UI speed control in its stable three-state cycle.
    pub const fn cycle(self) -> Self {
        match self {
            Self::Slow => Self::Normal,
            Self::Normal => Self::Fast,
            Self::Fast => Self::Slow,
        }
    }

    /// The speeds in control order, useful for a settings row or focused
    /// control without duplicating the ordering in UI code.
    pub const fn all() -> [Self; 3] {
        [Self::Slow, Self::Normal, Self::Fast]
    }

    fn from_saved_value(value: Value) -> Self {
        match value {
            Value::String(raw) => Self::from_saved_string(&raw),
            Value::Number(number) => number
                .as_f64()
                .and_then(Self::from_multiplier)
                .unwrap_or(Self::Normal),
            // `null`, booleans, arrays, and objects are unsupported preference
            // encodings.  They are recoverable, so retain the safe default.
            _ => Self::Normal,
        }
    }

    fn from_saved_string(raw: &str) -> Self {
        let normalized = raw.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "slow" => Self::Slow,
            "normal" => Self::Normal,
            "fast" => Self::Fast,
            _ => {
                // Accept the labels shown in the UI and a numeric string from
                // a hand-edited/older preference file.  Exact multiplier
                // matching intentionally rejects unsupported ordinals.
                let numeric = normalized.strip_suffix('x').unwrap_or(&normalized);
                numeric
                    .parse::<f64>()
                    .ok()
                    .and_then(Self::from_multiplier)
                    .unwrap_or(Self::Normal)
            }
        }
    }

    fn from_multiplier(value: f64) -> Option<Self> {
        if !value.is_finite() {
            return None;
        }
        if value == Self::Slow.multiplier() {
            Some(Self::Slow)
        } else if value == Self::Normal.multiplier() {
            Some(Self::Normal)
        } else if value == Self::Fast.multiplier() {
            Some(Self::Fast)
        } else {
            None
        }
    }
}

impl<'de> Deserialize<'de> for GameSpeed {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self::from_saved_value(Value::deserialize(deserializer)?))
    }
}

/// Release due fixed simulation ticks from a wall-clock accumulator.
///
/// `elapsed_seconds` is scaled by `speed.multiplier()`, then queued time is
/// capped at [`MAX_CATCH_UP_TICKS`] fixed steps.  The returned count is at
/// most six; subtracting it leaves only a sub-tick fractional remainder in
/// `accumulator`.  Invalid or negative elapsed values and a corrupted
/// accumulator are treated as zero so they cannot poison future scheduling.
pub fn schedule_ticks(accumulator: &mut f64, elapsed_seconds: f64, speed: GameSpeed) -> u32 {
    let previous = if accumulator.is_finite() && *accumulator > 0.0 {
        *accumulator
    } else {
        0.0
    };
    let pending = if elapsed_seconds.is_finite() && elapsed_seconds > 0.0 {
        let elapsed = elapsed_seconds * speed.multiplier();
        // A very large but finite duration can overflow while applying the
        // multiplier. Treat that as an over-cap stall so it still releases the
        // bounded maximum rather than silently scheduling no work.
        if elapsed.is_finite() {
            (previous + elapsed).min(MAX_BACKLOG_SECONDS)
        } else {
            MAX_BACKLOG_SECONDS
        }
    } else {
        previous.min(MAX_BACKLOG_SECONDS)
    };
    let ticks = ((pending / SIMULATION_TICK_SECONDS).floor() as u32).min(MAX_CATCH_UP_TICKS);
    *accumulator = pending - ticks as f64 * SIMULATION_TICK_SECONDS;

    // Keep the public invariant strong even in the presence of floating
    // point rounding at the cap or tick boundary.
    if !accumulator.is_finite() || *accumulator < 0.0 {
        *accumulator = 0.0;
    } else if *accumulator >= SIMULATION_TICK_SECONDS {
        *accumulator = SIMULATION_TICK_SECONDS.next_down();
    }
    ticks
}

/// Clear pending fractional wall time when the match is paused, leaves the
/// match screen, or loses focus.  Callers should also reset their `last_time`
/// instant at the same boundary so paused time is not reintroduced next frame.
pub fn reset_accumulator(accumulator: &mut f64) {
    *accumulator = 0.0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_multiplier_and_cycle_are_stable() {
        assert_eq!(GameSpeed::Slow.label(), "0.5X");
        assert_eq!(GameSpeed::Normal.label(), "1X");
        assert_eq!(GameSpeed::Fast.label(), "1.5X");
        assert_eq!(GameSpeed::Slow.multiplier(), 0.5);
        assert_eq!(GameSpeed::Normal.multiplier(), 1.0);
        assert_eq!(GameSpeed::Fast.multiplier(), 1.5);
        assert_eq!(GameSpeed::Slow.cycle(), GameSpeed::Normal);
        assert_eq!(GameSpeed::Normal.cycle(), GameSpeed::Fast);
        assert_eq!(GameSpeed::Fast.cycle(), GameSpeed::Slow);
        assert_eq!(
            GameSpeed::all(),
            [GameSpeed::Slow, GameSpeed::Normal, GameSpeed::Fast]
        );
    }

    #[test]
    fn one_fixed_tick_is_due_at_each_speed_period() {
        for speed in GameSpeed::all() {
            let mut accumulator = 0.0;
            let wall_period = SIMULATION_TICK_SECONDS / speed.multiplier();
            assert_eq!(schedule_ticks(&mut accumulator, wall_period, speed), 1);
            assert!(accumulator.abs() < 1.0e-12);
        }
    }

    #[test]
    fn sub_tick_elapsed_accumulates_at_each_speed() {
        for speed in GameSpeed::all() {
            let mut accumulator = 0.0;
            let half_period = SIMULATION_TICK_SECONDS / speed.multiplier() / 2.0;
            assert_eq!(schedule_ticks(&mut accumulator, half_period, speed), 0);
            assert!(accumulator > 0.0 && accumulator < SIMULATION_TICK_SECONDS);
            assert_eq!(schedule_ticks(&mut accumulator, half_period, speed), 1);
            assert!(accumulator.abs() < 1.0e-12);
        }
    }

    #[test]
    fn excessive_elapsed_is_capped_and_does_not_leave_backlog() {
        let mut accumulator = 0.0;
        assert_eq!(
            schedule_ticks(&mut accumulator, 60.0, GameSpeed::Fast),
            MAX_CATCH_UP_TICKS
        );
        assert!(accumulator.abs() < 1.0e-12);
        assert_eq!(
            schedule_ticks(&mut accumulator, f64::MAX, GameSpeed::Fast),
            MAX_CATCH_UP_TICKS
        );
        assert!(accumulator.abs() < 1.0e-12);
        assert_eq!(schedule_ticks(&mut accumulator, 0.0, GameSpeed::Normal), 0);
        assert!(accumulator.abs() < 1.0e-12);
    }

    #[test]
    fn reset_discards_fractional_time_for_pause_and_resume() {
        let mut accumulator = 0.0;
        let _ = schedule_ticks(
            &mut accumulator,
            SIMULATION_TICK_SECONDS * 0.75,
            GameSpeed::Normal,
        );
        assert!(accumulator > 0.0);
        reset_accumulator(&mut accumulator);
        assert_eq!(accumulator, 0.0);
        assert_eq!(
            schedule_ticks(
                &mut accumulator,
                SIMULATION_TICK_SECONDS * 0.5,
                GameSpeed::Normal
            ),
            0
        );
    }

    #[test]
    fn invalid_elapsed_or_accumulator_cannot_poison_future_ticks() {
        for elapsed in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut accumulator = 0.0;
            assert_eq!(
                schedule_ticks(&mut accumulator, elapsed, GameSpeed::Normal),
                0
            );
            assert_eq!(accumulator, 0.0);
        }

        let mut accumulator = f64::NAN;
        assert_eq!(schedule_ticks(&mut accumulator, 0.0, GameSpeed::Normal), 0);
        assert_eq!(accumulator, 0.0);
        accumulator = -1.0;
        assert_eq!(schedule_ticks(&mut accumulator, 0.0, GameSpeed::Normal), 0);
        assert_eq!(accumulator, 0.0);
    }

    #[test]
    fn unsupported_saved_values_fall_back_without_rejecting_preferences() {
        for raw in [
            r#"{"game_speed":"Turbo"}"#,
            r#"{"game_speed":99}"#,
            r#"{"game_speed":null}"#,
            r#"{"game_speed":{"name":"Fast"}}"#,
        ] {
            let preferences: crate::ux::Preferences =
                serde_json::from_str(raw).expect("recoverable preference");
            assert_eq!(preferences.game_speed, GameSpeed::Normal);
        }
    }

    #[test]
    fn legacy_missing_speed_defaults_and_new_preferences_serialize_canonically() {
        let legacy: crate::ux::Preferences =
            serde_json::from_str(r#"{"muted":true}"#).expect("legacy preference");
        assert!(legacy.muted);
        assert_eq!(legacy.game_speed, GameSpeed::Normal);
        assert_eq!(
            serde_json::to_value(&legacy).expect("preference serialization")["game_speed"],
            serde_json::json!("Normal")
        );

        for (raw, expected) in [
            (r#"{"game_speed":"slow"}"#, GameSpeed::Slow),
            (r#"{"game_speed":"0.5X"}"#, GameSpeed::Slow),
            (r#"{"game_speed":0.5}"#, GameSpeed::Slow),
            (r#"{"game_speed":"1X"}"#, GameSpeed::Normal),
            (r#"{"game_speed":1.0}"#, GameSpeed::Normal),
            (r#"{"game_speed":"FAST"}"#, GameSpeed::Fast),
            (r#"{"game_speed":"1.5X"}"#, GameSpeed::Fast),
            (r#"{"game_speed":1.5}"#, GameSpeed::Fast),
        ] {
            let preferences: crate::ux::Preferences =
                serde_json::from_str(raw).expect("known preference");
            assert_eq!(preferences.game_speed, expected);
        }
    }
}
