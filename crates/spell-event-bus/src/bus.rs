use crate::api::*;
use async_std::sync::Arc;
use async_std::task;
use eyre::eyre;
use fluence_libp2p::types::{Inlet, Outlet};
use futures::stream;
use futures::stream::BoxStream;
use futures::{channel::mpsc::unbounded, select, StreamExt};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::time::{Duration, Instant};

struct Subscribers {
    subscribers: HashMap<PeerEventType, Vec<Arc<Spell>>>,
}

impl Subscribers {
    fn new() -> Self {
        Self {
            subscribers: HashMap::new(),
        }
    }

    fn add(&mut self, spell_id: Arc<Spell>, event_types: Vec<PeerEventType>) {
        for event_type in event_types {
            self.subscribers
                .entry(event_type)
                .or_default()
                .push(spell_id.clone());
        }
    }

    fn get(&self, event_type: &PeerEventType) -> impl Iterator<Item = &Arc<Spell>> {
        self.subscribers
            .get(event_type)
            .map(|x| x.iter())
            .unwrap_or_else(|| [].iter())
    }

    fn remove(&mut self, spell_id: SpellId) {
        for subscribers in self.subscribers.values_mut() {
            subscribers.retain(|sub| sub.id != spell_id);
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Periodic<T> {
    pub id: T,
    pub period: Duration,
}

#[derive(Debug, PartialEq, Eq)]
struct Scheduled<T> {
    data: Periodic<T>,
    // the time after which we need to notify the subscriber
    run_at: Instant,
}

impl<T: Eq> Scheduled<T> {
    fn at(data: Periodic<T>, now: Instant) -> Option<Scheduled<T>> {
        let run_at = now.checked_add(data.period)?;
        Some(Scheduled { data, run_at })
    }

    fn reschedule(mut self, now: Instant) -> Option<Scheduled<T>> {
        self.run_at = now.checked_add(self.data.period)?;
        Some(self)
    }
}

// Implement it this way for min heap
impl<T: PartialEq + Eq> Ord for Scheduled<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        other.run_at.cmp(&self.run_at)
    }
}

impl<T: Eq> PartialOrd for Scheduled<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct SubscribersState {
    subscribers: Subscribers,
    scheduled: BinaryHeap<Scheduled<Arc<Spell>>>,
}

impl SubscribersState {
    fn new() -> Self {
        Self {
            subscribers: Subscribers::new(),
            scheduled: BinaryHeap::new(),
        }
    }

    fn subscribe(&mut self, spell: Spell, config: TriggersConfig) -> eyre::Result<()> {
        let spell = Arc::new(spell);
        for config in config.triggers {
            match config {
                TriggerConfig::Timer(config) => {
                    let periodic = Periodic {
                        id: spell.clone(),
                        period: config.period,
                    };
                    let scheduled = Scheduled::at(periodic, Instant::now())
                        .ok_or_else(|| eyre!("time overflow"))?;
                    self.scheduled.push(scheduled);
                }
                TriggerConfig::PeerEvent(config) => {
                    self.subscribers.add(spell.clone(), config.events);
                }
            }
        }
        Ok(())
    }

    fn unsubscribe(&mut self, spell_id: SpellId) {
        self.scheduled
            .retain(|scheduled| scheduled.data.id.id != spell_id);
        self.subscribers.remove(spell_id);
    }

    fn subscribers(&self, event_type: &PeerEventType) -> impl Iterator<Item = &Arc<Spell>> {
        self.subscribers.get(event_type)
    }

    fn next_scheduled_in(&self, now: Instant) -> Option<Duration> {
        self.scheduled
            .peek()
            .map(|scheduled| scheduled.run_at.saturating_duration_since(now))
    }
}

pub struct SpellEventBus {
    // List of events producers.
    sources: Vec<BoxStream<'static, PeerEvent>>,
    // API connections
    recv_cmd_channel: Inlet<Command>,
    // Notify when event to which a spell subscribed happened.
    send_events: Outlet<SpellEvent>,
}

impl SpellEventBus {
    pub fn new(
        sources: Vec<BoxStream<'static, PeerEvent>>,
    ) -> (Self, SpellEventBusApi, Inlet<SpellEvent>) {
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
                async_std::stream::interval(next_scheduled_in).fuse()
            };

            let result: eyre::Result<()> = try {
                select! {
                    command = recv_cmd_channel.select_next_some() => {
                        match command {
                            Command::Subscribe(spell, config) => {
                                state.subscribe(spell, config).map_err(|e| eyre!("Can't subscribe spell: {e}"))?;
                            },
                            Command::Unsubscribe(spell_id, reply) => {
                                state.unsubscribe(spell_id);
                                reply.send(()).map_err(|e| eyre!("Can't send notification about spell removal: {:?}", e))?;
                            },
                        }
                    },
                    event = sources_channel.select_next_some() => {
                        for spell in state.subscribers(&event.get_type()) {
                            send_events.unbounded_send(SpellEvent { id: spell.id.clone(), event: Event::Peer(event.clone()) })
                                       .map_err(|e| eyre!("Can't send notification about event {:?}: {:?}", event, e))?;
                        }
                    },
                    _ = timer.select_next_some() => {
                        // The timer is triggered only if there are some spells to be awaken.
                        let scheduled_spell = state.scheduled.pop().ok_or_else(eyre!("billions of years have gone by already?"));
                        send_events.unbounded_send(SpellEvent { id: scheduled_spell.data.id.id.clone(), event: Event::Timer })
                                   .map_err(|e| eyre!("Can't send notification by timer: {:?}", e))?;
                        let rescheduled = scheduled_spell.reschedule(now)
                                                         .ok_or_else(|| eyre!("Can't reschedule periodic spell"))?;
                        state.scheduled.push(rescheduled);
                    },
                }
            };
            if let Err(e) = result {
                log::error!("{}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::api::*;
    use crate::bus::*;
    use futures::StreamExt;
    use std::assert_matches::assert_matches;
    use std::time::Duration;

    #[test]
    fn test_timer() {
        use async_std::task;

        let (bus, api, event_stream) = SpellEventBus::new(vec![]);
        bus.start();

        let spell1_id = "spell1".to_string();
        let spell2_id = "spell2".to_string();
        let spell1_period = Duration::from_millis(5);
        let spell2_period = Duration::from_secs(10);
        api.subscribe(
            Spell {
                id: spell1_id.clone(),
            },
            TriggersConfig {
                triggers: vec![TriggerConfig::Timer(TimerConfig {
                    period: spell1_period,
                })],
            },
        )
        .unwrap();
        api.subscribe(
            Spell {
                id: spell2_id.clone(),
            },
            TriggersConfig {
                triggers: vec![TriggerConfig::Timer(TimerConfig {
                    period: spell2_period,
                })],
            },
        )
        .unwrap();

        // let's remove spell2
        task::block_on(async { api.unsubscribe(spell2_id).await }).unwrap();

        // let's collect 5 more events from spell1
        let events =
            task::block_on(async { event_stream.take(5).collect::<Vec<SpellEvent>>().await });
        assert_eq!(events.len(), 5);
        for event in events.into_iter() {
            assert_eq!(event.id, spell1_id.clone(),);
            assert_matches!(event.event, Event::Timer);
        }
    }
}
