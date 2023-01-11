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
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;

struct PeerEventSubscribers {
    subscribers: HashMap<PeerEventType, Vec<Arc<SpellId>>>,
}

impl PeerEventSubscribers {
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

    /// Returns true if spell_id was removed from subscribers
    fn remove(&mut self, spell_id: &SpellId) -> bool {
        let mut was_removed = false;
        for subscribers in self.subscribers.values_mut() {
            let subscribers_len = subscribers.len();
            subscribers.retain(|sub_id| **sub_id != *spell_id);
            was_removed |= subscribers_len != subscribers.len();
        }
        was_removed
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

    /// Reschedule a spell to `now` + `period`.
    /// Return `None` if the spell is supposed to end at the given time `end_at`.
    fn at(data: Periodic, now: Instant) -> Option<Scheduled> {
        if data.end_at.map(|end_at| end_at <= now).unwrap_or(false) {
            return None;
        }

        // We do checked_add here only to avoid a mere possibility of internal panic.
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
    subscribers: PeerEventSubscribers,
    scheduled: BinaryHeap<Scheduled>,
}

impl SubscribersState {
    fn new() -> Self {
        Self {
            subscribers: PeerEventSubscribers::new(),
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

    /// Returns true if spell_id was removed from subscribers
    fn unsubscribe(&mut self, spell_id: &SpellId) -> bool {
        let prev_len = self.scheduled.len();
        self.scheduled
            .retain(|scheduled| *scheduled.data.id != *spell_id);
        let new_len = self.scheduled.len();
        let removed = self.subscribers.remove(spell_id);
        prev_len != new_len || removed
    }

    fn update(&mut self, spell_id: &SpellId, config: &SpellTriggerConfigs) {
        if self.unsubscribe(spell_id) {
            self.subscribe(spell_id.clone(), config);
        }
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
    /// Notify when trigger happened
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
                let next_scheduled_in = state.next_scheduled_in(now);
                if next_scheduled_in.is_some() {
                    log::trace!("Next time trigger will execute in: {:?}", next_scheduled_in);
                    log::trace!("Scheduled triggers: {:?}", state.scheduled);
                }
                // If there's no more scheduled triggers, then use Duration::MAX so that timer will
                // not fire, until there's a new TimeTrigger config
                let interval = next_scheduled_in.unwrap_or(Duration::MAX);
                async_std::stream::interval(interval).fuse()
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
                            Action::Update(config) => {
                                state.update(&spell_id, config);
                            }
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
                            let timestamp_secs = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs();
                            Self::trigger_spell(&send_events, &scheduled_spell.data.id, Event::Timer(timestamp_secs))?;
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
                spell_id: (**id).clone(),
                event: event.clone(),
            })
            .map_err(|e| BusInternalError::SendEvent((**id).clone(), event, e.into_send_error()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::bus::*;
    use async_std::task::JoinHandle;
    use connection_pool::LifecycleEvent;
    use futures::StreamExt;
    use libp2p::PeerId;
    use maplit::hashmap;
    use particle_protocol::Contact;
    use std::assert_matches::assert_matches;
    use std::time::Duration;

    // Safely call teardown after test.
    fn try_catch<T>(test: T, teardown: impl FnOnce())
    where
        T: FnOnce() + std::panic::UnwindSafe,
    {
        let err = std::panic::catch_unwind(test);

        teardown();

        if let Err(err) = err {
            std::panic::resume_unwind(err);
        }
    }

    fn send_connect_event(send: &Outlet<PeerEvent>, peer_id: PeerId) {
        send.unbounded_send(PeerEvent::ConnectionPool(LifecycleEvent::Connected(
            Contact::new(peer_id, Vec::new()),
        )))
        .unwrap();
    }

    fn send_disconnect_event(send: &Outlet<PeerEvent>, peer_id: PeerId) {
        send.unbounded_send(PeerEvent::ConnectionPool(LifecycleEvent::Disconnected(
            Contact::new(peer_id, Vec::new()),
        )))
        .unwrap();
    }

    fn emulate_connect(period: Duration) -> (Inlet<PeerEvent>, JoinHandle<()>) {
        let (send, recv) = unbounded();
        let hdl = task::spawn(async move {
            let mut interval = async_std::stream::interval(period).fuse();
            loop {
                select! {
                    _ = interval.select_next_some() => {
                        send_connect_event(&send, PeerId::random());
                    }
                }
            }
        });
        (recv, hdl)
    }

    fn subscribe_peer_event(api: &SpellEventBusApi, spell_id: SpellId, events: Vec<PeerEventType>) {
        task::block_on(api.subscribe(
            spell_id,
            SpellTriggerConfigs {
                triggers: vec![TriggerConfig::PeerEvent(PeerEventConfig { events })],
            },
        ))
        .unwrap();
    }

    fn subscribe_timer(api: &SpellEventBusApi, spell_id: SpellId, config: TimerConfig) {
        task::block_on(api.subscribe(
            spell_id,
            SpellTriggerConfigs {
                triggers: vec![TriggerConfig::Timer(config)],
            },
        ))
        .unwrap();
    }

    fn subscribe_periodic_endless(api: &SpellEventBusApi, spell_id: SpellId, period: Duration) {
        subscribe_timer(
            api,
            spell_id,
            TimerConfig::periodic(period, Instant::now(), None),
        );
    }

    #[test]
    fn test_subscribe_one() {
        let (bus, api, event_stream) = SpellEventBus::new(vec![]);
        let bus = bus.start();

        let spell1_id = "spell1".to_string();
        subscribe_periodic_endless(&api, spell1_id.clone(), Duration::from_millis(5));

        let events =
            task::block_on(async { event_stream.take(5).collect::<Vec<TriggerEvent>>().await });
        try_catch(
            || {
                assert_eq!(events.len(), 5);
                for event in events.into_iter() {
                    assert_eq!(event.spell_id, spell1_id.clone(),);
                    assert_matches!(event.event, Event::Timer);
                }
            },
            || {
                task::block_on(async {
                    bus.cancel().await;
                });
            },
        );
    }

    #[test]
    fn test_subscribe_many() {
        let (bus, api, event_stream) = SpellEventBus::new(vec![]);
        let bus = bus.start();

        let mut spell_ids = hashmap![
            "spell1".to_string() => 0,
            "spell2".to_string() => 0,
            "spell3".to_string() => 0,
        ];
        for spell_id in spell_ids.keys() {
            subscribe_periodic_endless(&api, spell_id.clone(), Duration::from_millis(5));
        }

        let events =
            task::block_on(async { event_stream.take(10).collect::<Vec<TriggerEvent>>().await });
        try_catch(
            move || {
                assert_eq!(events.len(), 10);
                for event in events.into_iter() {
                    spell_ids.entry(event.spell_id).and_modify(|e| *e += 1);
                    assert_matches!(event.event, Event::Timer);
                }
                for count in spell_ids.values() {
                    assert_ne!(*count, 0);
                }
            },
            || {
                task::block_on(async {
                    bus.cancel().await;
                });
            },
        );
    }

    #[test]
    fn test_subscribe_oneshot() {
        let (bus, api, event_stream) = SpellEventBus::new(vec![]);
        let bus = bus.start();
        let spell1_id = "spell1".to_string();
        subscribe_timer(
            &api,
            spell1_id.clone(),
            TimerConfig::oneshot(Instant::now()),
        );
        let spell2_id = "spell2".to_string();
        subscribe_periodic_endless(&api, spell2_id.clone(), Duration::from_millis(5));

        let events =
            task::block_on(async { event_stream.take(5).collect::<Vec<TriggerEvent>>().await });

        let mut counts = HashMap::new();
        counts.insert(spell1_id.clone(), 0);
        counts.insert(spell2_id.clone(), 0);
        for event in events.into_iter() {
            counts.entry(event.spell_id).and_modify(|e| *e += 1);
        }
        try_catch(
            || {
                assert_eq!(*counts.get(&spell1_id).unwrap(), 1);
            },
            || {
                task::block_on(async {
                    bus.cancel().await;
                });
            },
        );
    }

    #[test]
    fn test_subscribe_connect() {
        let (send, recv) = unbounded();
        let (bus, api, mut event_stream) = SpellEventBus::new(vec![recv.boxed()]);
        let bus = bus.start();

        let spell1_id = "spell1".to_string();
        subscribe_peer_event(&api, spell1_id.clone(), vec![PeerEventType::Connected]);

        let peer_id = PeerId::random();
        send_connect_event(&send, peer_id);

        let event = task::block_on(async { event_stream.next().await.unwrap() });
        try_catch(
            || {
                assert_eq!(event.spell_id, spell1_id.clone());
                let expected = Contact::new(peer_id, Vec::new());
                assert_matches!(
                    event.event,
                    Event::Peer(PeerEvent::ConnectionPool(LifecycleEvent::Connected(contact))) if contact == expected
                );
            },
            || {
                task::block_on(async {
                    bus.cancel().await;
                });
            },
        );
    }

    #[test]
    fn test_unsubscribe() {
        use async_std::task;

        let (send, recv) = unbounded();
        let (bus, api, mut event_stream) = SpellEventBus::new(vec![recv.boxed()]);
        let bus = bus.start();

        let spell1_id = "spell1".to_string();
        subscribe_peer_event(&api, spell1_id.clone(), vec![PeerEventType::Connected]);

        let spell2_id = "spell2".to_string();
        subscribe_peer_event(&api, spell2_id.clone(), vec![PeerEventType::Connected]);
        send_connect_event(&send, PeerId::random());

        task::block_on(async {
            let triggered_spell_a = event_stream.next().await.unwrap().spell_id;
            let triggered_spell_b = event_stream.next().await.unwrap().spell_id;
            let triggered = vec![triggered_spell_a, triggered_spell_b];
            assert!(triggered.contains(&spell1_id), "spell_1 must be triggered");
            assert!(triggered.contains(&spell2_id), "spell_2 must be triggered");

            api.unsubscribe(spell2_id).await.unwrap();
            send_connect_event(&send, PeerId::random());
            let triggered_spell_1 = event_stream.next().await.unwrap().spell_id;
            assert_eq!(spell1_id, triggered_spell_1);
            assert!(
                event_stream.try_next().is_err(),
                "no other spells must be triggered"
            );
        });

        task::block_on(async {
            bus.cancel().await;
        });
    }

    #[test]
    fn test_subscribe_many_spells_with_diff_event_types() {
        let (recv, hdl) = emulate_connect(Duration::from_millis(10));
        let (bus, api, event_stream) = SpellEventBus::new(vec![recv.boxed()]);
        let bus = bus.start();

        let spell1_id = "spell1".to_string();
        subscribe_peer_event(&api, spell1_id.clone(), vec![PeerEventType::Connected]);

        let spell2_id = "spell2".to_string();
        subscribe_periodic_endless(&api, spell2_id.clone(), Duration::from_millis(5));

        let events =
            task::block_on(async { event_stream.take(10).collect::<Vec<TriggerEvent>>().await });
        try_catch(
            || {
                for event in events.into_iter() {
                    assert!(event.spell_id == spell1_id || event.spell_id == spell2_id);
                    if event.spell_id == spell1_id {
                        assert_matches!(
                            event.event,
                            Event::Peer(PeerEvent::ConnectionPool(LifecycleEvent::Connected(_)))
                        );
                    } else if event.spell_id == spell2_id {
                        assert_matches!(event.event, Event::Timer);
                    }
                }
            },
            || {
                task::block_on(async {
                    hdl.cancel().await;
                    bus.cancel().await;
                });
            },
        );
    }

    #[test]
    fn test_update_config() {
        let (send, recv) = unbounded();
        let (bus, api, mut event_stream) = SpellEventBus::new(vec![recv.boxed()]);
        let _bus = bus.start();

        let spell1_id = "spell1".to_string();
        subscribe_peer_event(&api, spell1_id.clone(), vec![PeerEventType::Connected]);

        send_connect_event(&send, PeerId::random());
        send_disconnect_event(&send, PeerId::random());

        task::block_on(async {
            let connect_event = event_stream.next().await.unwrap();
            let disconnect_event = event_stream.try_next();
            assert_eq!(connect_event.spell_id, spell1_id);
            assert_matches!(
                connect_event.event,
                Event::Peer(PeerEvent::ConnectionPool(LifecycleEvent::Connected(_)))
            );
            assert!(
                disconnect_event.is_err(),
                "no spells are triggered by disconnect event"
            );

            let config = SpellTriggerConfigs {
                triggers: vec![TriggerConfig::PeerEvent(PeerEventConfig {
                    events: vec![PeerEventType::Disconnected],
                })],
            };
            api.update_config(spell1_id.clone(), config).await.unwrap();

            send_connect_event(&send, PeerId::random());
            send_disconnect_event(&send, PeerId::random());
            let disconnect_event = event_stream.next().await.unwrap();
            let connect_event = event_stream.try_next();
            assert_eq!(disconnect_event.spell_id, spell1_id);
            assert_matches!(
                disconnect_event.event,
                Event::Peer(PeerEvent::ConnectionPool(LifecycleEvent::Disconnected(_)))
            );
            assert!(
                connect_event.is_err(),
                "no spells are triggered by disconnect event"
            );
        });
    }
}
