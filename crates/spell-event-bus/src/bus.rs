use crate::api::*;
use crate::config::{SpellTriggerConfigs, TriggerConfig};
use async_std::sync::Arc;
use async_std::task;
use fluence_libp2p::types::{Inlet, Outlet};
use futures::channel::mpsc::SendError;
use futures::stream;
use futures::stream::BoxStream;
use futures::{channel::mpsc::unbounded, select, StreamExt};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::time::{Duration, Instant};
use thiserror::Error;

struct Subscribers {
    subscribers: HashMap<PeerEventType, Vec<Arc<SpellId>>>,
}

impl Subscribers {
    fn new() -> Self {
        Self {
            subscribers: HashMap::new(),
        }
    }

    fn add(&mut self, spell_id: Arc<SpellId>, event_types: Vec<PeerEventType>) {
        for event_type in event_types {
            self.subscribers
                .entry(event_type)
                .or_default()
                .push(spell_id.clone());
        }
    }

    fn get(&self, event_type: &PeerEventType) -> impl Iterator<Item = &Arc<SpellId>> {
        self.subscribers
            .get(event_type)
            .map(|x| x.iter())
            .unwrap_or_else(|| [].iter())
    }

    fn remove(&mut self, spell_id: &SpellId) {
        for subscribers in self.subscribers.values_mut() {
            subscribers.retain(|sub_id| **sub_id != *spell_id);
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Periodic {
    id: Arc<SpellId>,
    period: Duration,
    end_at: Option<Instant>,
}

#[derive(Debug, PartialEq, Eq)]
struct Scheduled {
    data: Periodic,
    /// the time after which we need to notify the subscriber
    run_at: Instant,
}

impl Scheduled {
    fn new(data: Periodic, run_at: Instant) -> Self {
        Self { data, run_at }
    }
    // schedule to no earlier than now + data.period
    fn at(data: Periodic, now: Instant) -> Option<Scheduled> {
        if data.end_at.map(|end_at| end_at <= now).unwrap_or(false) {
            return None;
        }
        let run_at = now.checked_add(data.period)?;
        Some(Scheduled { data, run_at })
    }
}

// Implement it this way for min heap
impl Ord for Scheduled {
    fn cmp(&self, other: &Self) -> Ordering {
        other.run_at.cmp(&self.run_at)
    }
}

impl PartialOrd for Scheduled {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct SubscribersState {
    subscribers: Subscribers,
    scheduled: BinaryHeap<Scheduled>,
}

impl SubscribersState {
    fn new() -> Self {
        Self {
            subscribers: Subscribers::new(),
            scheduled: BinaryHeap::new(),
        }
    }

    fn subscribe(&mut self, spell_id: SpellId, config: &SpellTriggerConfigs) -> Option<()> {
        let spell_id = Arc::new(spell_id);
        for config in &config.triggers {
            match config {
                TriggerConfig::Timer(config) => {
                    let periodic = Periodic {
                        id: spell_id.clone(),
                        period: config.period,
                        end_at: config.end_at,
                    };
                    let scheduled = Scheduled::new(periodic, config.start_at);
                    self.scheduled.push(scheduled);
                }
                TriggerConfig::PeerEvent(config) => {
                    self.subscribers
                        .add(spell_id.clone(), config.events.clone());
                }
            }
        }
        Some(())
    }

    fn unsubscribe(&mut self, spell_id: &SpellId) {
        self.scheduled
            .retain(|scheduled| *scheduled.data.id != *spell_id);
        self.subscribers.remove(spell_id);
    }

    fn subscribers(&self, event_type: &PeerEventType) -> impl Iterator<Item = &Arc<SpellId>> {
        self.subscribers.get(event_type)
    }

    fn next_scheduled_in(&self, now: Instant) -> Option<Duration> {
        self.scheduled
            .peek()
            .map(|scheduled| scheduled.run_at.saturating_duration_since(now))
    }
}

#[derive(Debug, Error)]
enum BusInternalError {
    // oneshot::Sender doesn't provide the reasons why it failed to send a message
    #[error("failed to send a result of a command execution ({1:?}) for a spell {0}: receiving end probably dropped")]
    Reply(SpellId, Action),
    #[error("failed to send notification about a peer event {1:?} to spell {0}: {2}")]
    SendEvent(SpellId, Event, SendError),
}

pub struct SpellEventBus {
    /// List of events producers.
    sources: Vec<BoxStream<'static, PeerEvent>>,
    /// API connections
    recv_cmd_channel: Inlet<Command>,
    /// Notify when event to which a spell subscribed happened.
    send_events: Outlet<TriggerEvent>,
}

impl SpellEventBus {
    pub fn new(
        sources: Vec<BoxStream<'static, PeerEvent>>,
    ) -> (Self, SpellEventBusApi, Inlet<TriggerEvent>) {
        let (send_cmd_channel, recv_cmd_channel) = unbounded();
        let api = SpellEventBusApi { send_cmd_channel };

        let (send_events, recv_events) = unbounded();

        let this = Self {
            sources,
            recv_cmd_channel,
            send_events,
        };
        (this, api, recv_events)
    }

    pub fn start(self) -> task::JoinHandle<()> {
        task::spawn(self.run())
    }

    async fn run(self) {
        let send_events = self.send_events;

        let mut recv_cmd_channel = self.recv_cmd_channel.fuse();
        let sources = self
            .sources
            .into_iter()
            .map(|source| source.fuse())
            .collect::<Vec<_>>();
        let mut sources_channel = stream::select_all(sources);

        let mut state = SubscribersState::new();
        loop {
            let now = Instant::now();
            // Wait until the next spell should be awaken. If there are no spells wait for unreachable amount of time,
            // which means that timer won't be triggered at all. We overwrite the timer each loop (aka after each event)
            // to ensure that we don't miss newly scheduled spells.
            let mut timer = {
                let next_scheduled_in = state.next_scheduled_in(now).unwrap_or(Duration::MAX);
                log::trace!("Next scheduled in: {:?}", next_scheduled_in);
                log::trace!("Scheduled: {:?}", state.scheduled);
                async_std::stream::interval(next_scheduled_in).fuse()
            };

            let result: Result<(), BusInternalError> = try {
                select! {
                    command = recv_cmd_channel.select_next_some() => {
                        let Command { spell_id, action, reply } = command;
                        match &action {
                            Action::Subscribe(config) => {
                                state.subscribe(spell_id.clone(), &config).unwrap_or(());
                            },
                            Action::Unsubscribe => {
                                state.unsubscribe(&spell_id);
                            },
                        };
                        reply.send(()).map_err(|_| BusInternalError::Reply(spell_id, action))?;
                    },
                    event = sources_channel.select_next_some() => {
                        for spell_id in state.subscribers(&event.get_type()) {
                            let event = Event::Peer(event.clone());
                            Self::trigger_spell(&send_events, spell_id, event)?;
                        }
                    },
                    _ = timer.select_next_some() => {
                        // The timer is triggered only if there are some spells to be awaken.
                        if let Some(scheduled_spell) = state.scheduled.pop() {
                            log::trace!("Execute: {:?}", scheduled_spell);
                            Self::trigger_spell(&send_events, &scheduled_spell.data.id, Event::Timer)?;
                            // Do not reschedule the spell otherwise.
                            if let Some(rescheduled) = Scheduled::at(scheduled_spell.data, Instant::now()) {
                                log::trace!("Reschedule: {:?}", rescheduled);
                                state.scheduled.push(rescheduled);
                            }
                        }
                    },
                }
            };
            if let Err(e) = result {
                log::warn!("Error in spell event bus loop: {}", e);
            }
        }
    }

    fn trigger_spell(
        send_events: &Outlet<TriggerEvent>,
        id: &Arc<SpellId>,
        event: Event,
    ) -> Result<(), BusInternalError> {
        send_events
            .unbounded_send(TriggerEvent {
                id: (**id).clone(),
                event: event.clone(),
            })
            .map_err(|e| BusInternalError::SendEvent((**id).clone(), event, e.into_send_error()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::bus::*;
    use futures::StreamExt;
    use std::assert_matches::assert_matches;
    use std::time::Duration;

    #[test]
    fn test_timer() {
        use crate::config::TimerConfig;
        use async_std::task;

        let (bus, api, event_stream) = SpellEventBus::new(vec![]);
        bus.start();

        let spell1_id = "spell1".to_string();
        let spell2_id = "spell2".to_string();
        let spell1_period = Duration::from_millis(5);
        let spell2_period = Duration::from_secs(10);
        task::block_on(api.subscribe(
            spell1_id.clone(),
            SpellTriggerConfigs {
                triggers: vec![TriggerConfig::Timer(TimerConfig {
                    period: spell1_period,
                    start_at: Instant::now(),
                    end_at: None,
                })],
            },
        ))
        .unwrap();
        task::block_on(api.subscribe(
            spell2_id.clone(),
            SpellTriggerConfigs {
                triggers: vec![TriggerConfig::Timer(TimerConfig {
                    period: spell2_period,
                    start_at: Instant::now(),
                    end_at: None,
                })],
            },
        ))
        .unwrap();

        // let's remove spell2
        task::block_on(async { api.unsubscribe(spell2_id).await }).unwrap();

        // let's collect 5 more events from spell1
        let events =
            task::block_on(async { event_stream.take(5).collect::<Vec<TriggerEvent>>().await });
        assert_eq!(events.len(), 5);
        for event in events.into_iter() {
            assert_eq!(event.id, spell1_id.clone(),);
            assert_matches!(event.event, Event::Timer);
        }
    }
}
