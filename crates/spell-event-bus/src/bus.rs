use crate::api::*;
use crate::config::{SpellTriggerConfigs, TriggerConfig};
use futures::stream::BoxStream;
use futures::StreamExt;
use futures::{future, FutureExt};
use peer_metrics::SpellMetrics;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::select;
use tokio::sync::mpsc;
use tokio::task;

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

    /// Reschedule a spell to `now` + `period`.
    /// Return `None` if the spell is supposed to end at the given time `end_at`.
    fn at(data: Periodic, now: Instant) -> Option<Scheduled> {
        // We do checked_add here only to avoid a mere possibility of internal panic.
        let run_at = now.checked_add(data.period)?;
        if data.end_at.map(|end_at| end_at <= run_at).unwrap_or(false) {
            return None;
        }

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
    #[error(
        "failed to send a result of a command execution ({0:?}): receiving end probably dropped"
    )]
    Reply(Action),
    #[error("failed to send notification about a peer event {1:?} to spell {0}: {2}")]
    SendEvent(SpellId, TriggerInfo, Pin<Box<dyn std::error::Error>>),
}

pub struct SpellEventBus {
    /// List of events producers.
    sources: Vec<BoxStream<'static, PeerEvent>>,
    /// API connections
    recv_cmd_channel: mpsc::UnboundedReceiver<Command>,
    /// Notify when trigger happened
    send_events: mpsc::UnboundedSender<TriggerEvent>,
    /// Spell metrics
    spell_metrics: Option<SpellMetrics>,
}

impl SpellEventBus {
    pub fn new(
        spell_metrics: Option<SpellMetrics>,
        sources: Vec<BoxStream<'static, PeerEvent>>,
    ) -> (
        Self,
        SpellEventBusApi,
        mpsc::UnboundedReceiver<TriggerEvent>,
    ) {
        let (send_cmd_channel, recv_cmd_channel) = mpsc::unbounded_channel();
        let api = SpellEventBusApi { send_cmd_channel };

        let (send_events, recv_events) = mpsc::unbounded_channel();

        let this = Self {
            sources,
            recv_cmd_channel,
            send_events,
            spell_metrics,
        };
        (this, api, recv_events)
    }

    pub fn start(self) -> task::JoinHandle<()> {
        task::Builder::new()
            .name("spell-bus")
            .spawn(self.run())
            .expect("Could not spawn task")
    }

    async fn run(mut self) {
        let send_events = self.send_events;

        let sources = self
            .sources
            .into_iter()
            .map(|source| source.fuse())
            .collect::<Vec<_>>();
        let mut sources_channel = futures::stream::select_all(sources);

        let mut state = SubscribersState::new();
        let mut is_started = false;
        loop {
            let now = Instant::now();

            // Wait until the next spell should be awaken. If there are no spells wait for unreachable amount of time,
            // which means that timer won't be triggered at all. We overwrite the timer each loop (aka after each event)
            // to ensure that we don't miss newly scheduled spells.
            let timer_task = {
                let next_scheduled_in = state.next_scheduled_in(now);
                if next_scheduled_in.is_some() {
                    log::trace!("Next time trigger will execute in: {:?}", next_scheduled_in);
                    log::trace!("Scheduled triggers: {:?}", state.scheduled);
                }
                next_scheduled_in
                    .map(|duration| {
                        if duration > Duration::ZERO {
                            tokio::time::sleep(duration).boxed()
                        } else {
                            future::ready(()).boxed()
                        }
                    })
                    .unwrap_or_else(|| future::pending::<()>().boxed())
            };

            let result: Result<(), BusInternalError> = try {
                select! {
                    Some(command) = self.recv_cmd_channel.recv() => {
                        let Command { action, reply } = command;
                        match &action {
                            Action::Subscribe(spell_id, config) => {
                                log::trace!("Subscribe {spell_id} to {:?}", config);
                                state.subscribe(spell_id.clone(), config).unwrap_or(());
                            },
                            Action::Unsubscribe(spell_id) => {
                                log::trace!("Unsubscribe {spell_id}");
                                state.unsubscribe(spell_id);
                            },
                            Action::Start => {
                                log::trace!("Start the bus");
                                is_started = true;
                            }
                        };
                        reply.send(()).map_err(|_| {
                            BusInternalError::Reply(action)
                        })?;
                    },
                    Some(event) = sources_channel.next(), if is_started => {
                        for spell_id in state.subscribers(&event.get_type()) {
                            let event = TriggerInfo::Peer(event.clone());
                            Self::trigger_spell(&send_events, spell_id, event)?;
                        }
                    },
                    _ = timer_task, if is_started => {
                        // The timer is triggered only if there are some spells to be awaken.
                        if let Some(scheduled_spell) = state.scheduled.pop() {
                            log::trace!("Execute: {:?}", scheduled_spell);
                            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs();
                            Self::trigger_spell(&send_events, &scheduled_spell.data.id, TriggerInfo::Timer(TimerEvent{ timestamp }))?;
                            // Do not reschedule the spell otherwise.
                            if let Some(rescheduled) = Scheduled::at(scheduled_spell.data, Instant::now()) {
                                log::trace!("Reschedule: {:?}", rescheduled);
                                state.scheduled.push(rescheduled);
                            } else if let Some(m) = &self.spell_metrics {
                                m.observe_finished_spell();
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

    #[allow(clippy::result_large_err)]
    fn trigger_spell(
        send_events: &mpsc::UnboundedSender<TriggerEvent>,
        id: &Arc<SpellId>,
        event: TriggerInfo,
    ) -> Result<(), BusInternalError> {
        send_events
            .send(TriggerEvent {
                spell_id: (**id).clone(),
                info: event.clone(),
            })
            .map_err(|e| BusInternalError::SendEvent((**id).clone(), event, Box::pin(e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::bus::*;
    use connection_pool::LifecycleEvent;
    use futures::StreamExt;
    use libp2p::PeerId;
    use maplit::hashmap;
    use particle_protocol::Contact;
    use std::assert_matches::assert_matches;
    use std::time::Duration;
    use tokio::task::JoinHandle;
    use tokio_stream::wrappers::UnboundedReceiverStream;

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

    fn send_connect_event(sender: &mpsc::UnboundedSender<PeerEvent>, peer_id: PeerId) {
        sender
            .send(PeerEvent::from(LifecycleEvent::Connected(Contact::new(
                peer_id,
                Vec::new(),
            ))))
            .unwrap();
    }

    fn emulate_connect(period: Duration) -> (mpsc::UnboundedReceiver<PeerEvent>, JoinHandle<()>) {
        let (send, recv) = mpsc::unbounded_channel();
        let hdl = task::Builder::new()
            .name("emulate_connect")
            .spawn(async move {
                let mut task = Some(tokio::time::sleep(period).boxed());
                loop {
                    let task = task.as_mut().expect("Could not get task");
                    select! {
                        _ = task => {
                            send_connect_event(&send, PeerId::random());
                        }
                    }
                }
            })
            .expect("Could not spawn task");
        (recv, hdl)
    }

    async fn subscribe_peer_event(
        api: &SpellEventBusApi,
        spell_id: SpellId,
        events: Vec<PeerEventType>,
    ) {
        api.subscribe(
            spell_id,
            SpellTriggerConfigs {
                triggers: vec![TriggerConfig::PeerEvent(PeerEventConfig { events })],
            },
        )
        .await
        .expect("Could not subscribe peer event");
    }

    async fn subscribe_timer(api: &SpellEventBusApi, spell_id: SpellId, config: TimerConfig) {
        api.subscribe(
            spell_id,
            SpellTriggerConfigs {
                triggers: vec![TriggerConfig::Timer(config)],
            },
        )
        .await
        .expect("Could not subscribe timer");
    }

    async fn subscribe_periodic_endless(
        api: &SpellEventBusApi,
        spell_id: SpellId,
        period: Duration,
    ) {
        subscribe_timer(
            api,
            spell_id,
            TimerConfig::periodic(period, Instant::now(), None),
        )
        .await;
    }

    #[tokio::test]
    async fn test_subscribe_one() {
        let (bus, api, event_receiver) = SpellEventBus::new(None, vec![]);
        let bus = bus.start();
        let _ = api.start_scheduling().await;
        let event_stream = UnboundedReceiverStream::new(event_receiver);

        let spell1_id = "spell1".to_string();
        subscribe_periodic_endless(&api, spell1_id.clone(), Duration::from_millis(5)).await;

        let events = event_stream.take(5).collect::<Vec<TriggerEvent>>().await;
        try_catch(
            || {
                assert_eq!(events.len(), 5);
                for event in events.into_iter() {
                    assert_eq!(event.spell_id, spell1_id.clone(),);
                    assert_matches!(event.info, TriggerInfo::Timer(_));
                }
            },
            || {
                bus.abort();
            },
        );
    }

    #[tokio::test]
    async fn test_subscribe_many() {
        let (bus, api, event_receiver) = SpellEventBus::new(None, vec![]);
        let bus = bus.start();
        let _ = api.start_scheduling().await;
        let event_stream = UnboundedReceiverStream::new(event_receiver);

        let mut spell_ids = hashmap![
            "spell1".to_string() => 0,
            "spell2".to_string() => 0,
            "spell3".to_string() => 0,
        ];
        for spell_id in spell_ids.keys() {
            subscribe_periodic_endless(&api, spell_id.clone(), Duration::from_millis(5)).await;
        }

        let events = event_stream.take(10).collect::<Vec<TriggerEvent>>().await;
        try_catch(
            move || {
                assert_eq!(events.len(), 10);
                for event in events.into_iter() {
                    spell_ids.entry(event.spell_id).and_modify(|e| *e += 1);
                    assert_matches!(event.info, TriggerInfo::Timer(_));
                }
                for count in spell_ids.values() {
                    assert_ne!(*count, 0);
                }
            },
            || {
                bus.abort();
            },
        );
    }

    #[tokio::test]
    async fn test_subscribe_oneshot() {
        let (bus, api, event_receiver) = SpellEventBus::new(None, vec![]);
        let bus = bus.start();
        let _ = api.start_scheduling().await;
        let event_stream = UnboundedReceiverStream::new(event_receiver);
        let spell1_id = "spell1".to_string();
        subscribe_timer(
            &api,
            spell1_id.clone(),
            TimerConfig::oneshot(Instant::now()),
        )
        .await;
        let spell2_id = "spell2".to_string();
        subscribe_periodic_endless(&api, spell2_id.clone(), Duration::from_millis(5)).await;

        let events = event_stream.take(5).collect::<Vec<TriggerEvent>>().await;

        let mut counts = HashMap::new();
        counts.insert(spell1_id.clone(), 0);
        counts.insert(spell2_id, 0);
        for event in events.into_iter() {
            counts.entry(event.spell_id).and_modify(|e| *e += 1);
        }
        try_catch(
            || {
                assert_eq!(*counts.get(&spell1_id).unwrap(), 1);
            },
            || {
                bus.abort();
            },
        );
    }

    #[tokio::test]
    async fn test_subscribe_connect() {
        let (send, recv) = mpsc::unbounded_channel();
        let recv = UnboundedReceiverStream::new(recv).boxed();
        let (bus, api, event_receiver) = SpellEventBus::new(None, vec![recv]);
        let mut event_stream = UnboundedReceiverStream::new(event_receiver);
        let bus = bus.start();
        let _ = api.start_scheduling().await;

        let spell1_id = "spell1".to_string();
        subscribe_peer_event(&api, spell1_id.clone(), vec![PeerEventType::Connected]).await;

        let peer_id = PeerId::random();
        send_connect_event(&send, peer_id);

        let event = event_stream.next().await.unwrap();
        try_catch(
            || {
                assert_eq!(event.spell_id, spell1_id.clone());
                let expected = peer_id;
                assert_matches!(
                    event.info,
                    TriggerInfo::Peer(p) if p.peer_id == expected
                );
            },
            || {
                bus.abort();
            },
        );
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let (send, recv) = mpsc::unbounded_channel();
        let recv = UnboundedReceiverStream::new(recv).boxed();
        let (bus, api, mut event_receiver) = SpellEventBus::new(None, vec![recv]);
        let bus = bus.start();
        let _ = api.start_scheduling().await;

        let spell1_id = "spell1".to_string();
        subscribe_peer_event(&api, spell1_id.clone(), vec![PeerEventType::Connected]).await;

        let spell2_id = "spell2".to_string();
        subscribe_peer_event(&api, spell2_id.clone(), vec![PeerEventType::Connected]).await;
        send_connect_event(&send, PeerId::random());

        let triggered_spell_a = event_receiver.recv().await.unwrap().spell_id;
        let triggered_spell_b = event_receiver.recv().await.unwrap().spell_id;
        let triggered = vec![triggered_spell_a, triggered_spell_b];
        assert!(triggered.contains(&spell1_id), "spell_1 must be triggered");
        assert!(triggered.contains(&spell2_id), "spell_2 must be triggered");

        api.unsubscribe(spell2_id).await.unwrap();
        send_connect_event(&send, PeerId::random());
        let triggered_spell_1 = event_receiver.recv().await.unwrap().spell_id;
        assert_eq!(spell1_id, triggered_spell_1);
        let result = event_receiver.try_recv();
        assert!(result.is_err(), "no other spells must be triggered");
        bus.abort();
    }

    #[tokio::test]
    async fn test_subscribe_many_spells_with_diff_event_types() {
        let (recv, hdl) = emulate_connect(Duration::from_millis(10));
        let recv = UnboundedReceiverStream::new(recv).boxed();
        let (bus, api, event_receiver) = SpellEventBus::new(None, vec![recv]);
        let event_stream = UnboundedReceiverStream::new(event_receiver);
        let bus = bus.start();
        let _ = api.start_scheduling().await;

        let spell1_id = "spell1".to_string();
        subscribe_peer_event(&api, spell1_id.clone(), vec![PeerEventType::Connected]).await;

        let spell2_id = "spell2".to_string();
        subscribe_periodic_endless(&api, spell2_id.clone(), Duration::from_millis(5)).await;

        let events = event_stream.take(10).collect::<Vec<TriggerEvent>>().await;
        try_catch(
            || {
                for event in events.into_iter() {
                    assert!(event.spell_id == spell1_id || event.spell_id == spell2_id);
                    if event.spell_id == spell1_id {
                        assert_matches!(
                            event.info,
                           TriggerInfo::Peer(p) if p.connected
                        );
                    } else if event.spell_id == spell2_id {
                        assert_matches!(event.info, TriggerInfo::Timer(_));
                    }
                }
            },
            || {
                hdl.abort();
                bus.abort();
            },
        );
    }
}
