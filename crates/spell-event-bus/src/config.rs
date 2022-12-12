use crate::api::PeerEventType;
use fluence_spell_dtos::trigger_config::TriggerConfig as UserTriggerConfig;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;

const MAX_PERIOD_YEAR: u32 = 100;

// Set max period to 100 years in secs = 60 sec * 60 min * 24 hours * 365 days * 100 years
pub const MAX_PERIOD_SEC: u32 = 60 * 60 * 24 * 365 * MAX_PERIOD_YEAR;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(
        "invalid config: period is too big. Max period is {} years (or approx {} seconds)",
        MAX_PERIOD_YEAR,
        MAX_PERIOD_SEC
    )]
    InvalidPeriod,
    #[error("invalid config: end_sec is less than start_sec or in the past")]
    InvalidEndSec,
    #[error("config is empty, nothing to do")]
    EmptyConfig,
}

// The only way to convert timestamp to instant I found.
// Fails if the timestamp is in the past or overflow occurred which actually shouldn't happen.
fn to_instant(timestamp: u64) -> Option<Instant> {
    let target_time = UNIX_EPOCH.checked_add(Duration::from_secs(timestamp))?;
    let duration = target_time.duration_since(SystemTime::now()).ok()?;
    Instant::now().checked_add(duration)
}

/// Convert user-friendly config to event-bus-friendly config, validating it in the process.
pub fn from_user_config(
    user_config: UserTriggerConfig,
) -> Result<SpellTriggerConfigs, ConfigError> {
    let mut triggers = Vec::new();
    // Process timer config
    let clock = user_config.clock;
    if clock.start_sec != 0 {
        if clock.period_sec > MAX_PERIOD_SEC {
            return Err(ConfigError::InvalidPeriod);
        }

        let end_at = if clock.end_sec == 0 {
            None
        } else if clock.end_sec > clock.start_sec {
            match to_instant(clock.end_sec as u64) {
                Some(end_at) => Some(end_at),
                None => return Err(ConfigError::InvalidEndSec),
            }
        } else {
            return Err(ConfigError::InvalidEndSec);
        };

        // Start now if the start time is in the past
        let start_at = to_instant(clock.start_sec as u64).unwrap_or_else(Instant::now);

        // If period is 0, then the timer will be triggered only once at start_sec and then stopped.
        let end_at = if clock.period_sec == 0 {
            Some(start_at)
        } else {
            end_at
        };

        triggers.push(TriggerConfig::Timer(TimerConfig {
            period: Duration::from_secs(clock.period_sec as u64),
            start_at,
            end_at,
        }));
    }

    // Process connections config
    let mut pool_events = Vec::with_capacity(2);
    if user_config.connections.connect {
        pool_events.push(PeerEventType::Connected);
    }
    if user_config.connections.disconnect {
        pool_events.push(PeerEventType::Disconnected);
    }
    if !pool_events.is_empty() {
        triggers.push(TriggerConfig::PeerEvent(PeerEventConfig {
            events: pool_events,
        }));
    }

    if triggers.is_empty() {
        Err(ConfigError::EmptyConfig)
    } else {
        Ok(SpellTriggerConfigs { triggers })
    }
}

#[derive(Debug, Clone)]
pub struct SpellTriggerConfigs {
    pub(crate) triggers: Vec<TriggerConfig>,
}

#[derive(Debug, Clone)]
pub(crate) enum TriggerConfig {
    Timer(TimerConfig),
    PeerEvent(PeerEventConfig),
}

#[derive(Debug, Clone)]
pub(crate) struct TimerConfig {
    pub(crate) period: Duration,
    pub(crate) start_at: Instant,
    pub(crate) end_at: Option<Instant>,
}

#[derive(Debug, Clone)]
pub(crate) struct PeerEventConfig {
    pub(crate) events: Vec<PeerEventType>,
}
