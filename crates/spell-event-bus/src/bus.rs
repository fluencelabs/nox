use crate::api::*;
use async_std::sync::Arc;
use async_std::task;
use fluence_libp2p::types::{Inlet, Outlet};
use futures::stream;
use futures::{channel::mpsc::unbounded, select, StreamExt};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::time::{Duration, Instant};
use futures::stream::BoxStream;

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
    fn at(data: Periodic<T>, now: Instant) -> Scheduled<T> {
        let run_at = now.checked_add(data.period).expect("time overflow?");
        Scheduled { data, run_at }
    }

    fn reschedule(mut self, now: Instant) -> Scheduled<T> {
        self.run_at = now.checked_add(self.data.period).expect("time overflow?");
        self
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

    fn subscribe(&mut self, spell: Spell, config: TriggersConfig) {
        let spell = Arc::new(spell);
        for config in config.triggers {
            match config {
                TriggerConfig::Timer(config) => {
                    let periodic = Periodic { id: spell.clone(), period: config.period };
                    self.scheduled.push(Scheduled::at(periodic, Instant::now()))
                },
                TriggerConfig::PeerEvent(config) => {
                    self.subscribers.add(spell.clone(), config.events);
                },
            }
        }
    }

    fn unsubscribe(&mut self, spell_id: SpellId) {
        self.scheduled.retain(|scheduled| scheduled.data.id.id != spell_id);
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
    pub fn new(sources: Vec<BoxStream<'static, PeerEvent>>) -> (Self, SpellEventBusApi, Inlet<SpellEvent>) {
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

            select! {
                command = recv_cmd_channel.select_next_some() => {
                    match command {
                        Command::Subscribe(spell, config) => {
                            state.subscribe(spell, config);
                        },
                        Command::Unsubscribe(spell_id, reply) => {
                            state.unsubscribe(spell_id);
                            if let Err(e) = reply.send(()) {
                                log::warn!("Can't send notification about spell removal: {:?}", e);
                            }
                        },
                    }
                },
                event = sources_channel.select_next_some() => {
                    for spell in state.subscribers(&event.get_type()) {
                        if let Err(e) = send_events.unbounded_send(SpellEvent { id: spell.id.clone(), event: Event::Peer(event.clone()) }) {
                            log::warn!("Can't send notification about event {:?}: {:?}", event, e);
                        }
                    }
                },
                _ = timer.select_next_some() => {
                    // The timer is triggered only if there are some spells to be awaken.
                    let scheduled_spell = state.scheduled.pop().expect("billions of years have gone by already?");
                    if let Err(e) = send_events.unbounded_send(SpellEvent { id: scheduled_spell.data.id.id.clone(), event: Event::Timer }) {
                        log::warn!("Can't send notification by timer: {:?}", e);
                    }
                    state.scheduled.push(scheduled_spell.reschedule(now));
                },

            }
        }
    }
}
