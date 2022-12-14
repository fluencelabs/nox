use crate::api::PeerEventType;
use fluence_spell_dtos::trigger_config::{
    ClockConfig, ConnectionPoolConfig, TriggerConfig as UserTriggerConfig,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;

const MAX_PERIOD_YEAR: u32 = 100;

/// Max period is 100 years in secs: 60 sec * 60 min * 24 hours * 365 days * 100 years
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

/// Convert timestamp to std::time::Instant.
/// Fails if the timestamp is in the past or overflow occurred which actually shouldn't happen.
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

    // ClockConfig is considered empty if `start_sec` is zero. In this case the content of other fields are ignored.
    if user_config.clock.start_sec != 0 {
        let timer_config = from_clock_config(&user_config.clock)?;
        triggers.push(TriggerConfig::Timer(timer_config));
    }

    if let Some(peer_event_config) = from_connection_config(&user_config.connections) {
        triggers.push(TriggerConfig::PeerEvent(peer_event_config));
    }

    if triggers.is_empty() {
        Err(ConfigError::EmptyConfig)
    } else {
        Ok(SpellTriggerConfigs { triggers })
    }
}

fn from_connection_config(connection_config: &ConnectionPoolConfig) -> Option<PeerEventConfig> {
    let mut pool_events = Vec::with_capacity(2);
    if connection_config.connect {
        pool_events.push(PeerEventType::Connected);
    }
    if connection_config.disconnect {
        pool_events.push(PeerEventType::Disconnected);
    }
    if pool_events.is_empty() {
        None
    } else {
        Some(PeerEventConfig {
            events: pool_events,
        })
    }
}

fn from_clock_config(clock: &ClockConfig) -> Result<TimerConfig, ConfigError> {
    // Check the upper bound of period.
    if clock.period_sec > MAX_PERIOD_SEC {
        return Err(ConfigError::InvalidPeriod);
    }

    let end_at = if clock.end_sec == 0 {
        // If `end_sec` is 0 then the spell will be triggered forever.
        None
    } else if clock.end_sec < clock.start_sec {
        // The config is invalid `end_sec` is less than `start_sec`
        return Err(ConfigError::InvalidEndSec);
    } else {
        // If conversion fails that means that `end_sec` is in the past.
        match to_instant(clock.end_sec as u64) {
            Some(end_at) => Some(end_at),
            None => return Err(ConfigError::InvalidEndSec),
        }
    };

    // Start now if the start time is in the past
    let start_at = to_instant(clock.start_sec as u64).unwrap_or_else(Instant::now);

    // If period is 0, then the timer will be triggered only once at start_sec and then stopped.
    // So we set `end_at` to `start_at` to make sure that on rescheduling the spell will be stopped.
    // I'm not sure maybe it's better to move this piece of code inside the bus module.
    let end_at = if clock.period_sec == 0 {
        Some(start_at)
    } else {
        end_at
    };

    Ok(TimerConfig {
        period: Duration::from_secs(clock.period_sec as u64),
        start_at,
        end_at,
    })
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
