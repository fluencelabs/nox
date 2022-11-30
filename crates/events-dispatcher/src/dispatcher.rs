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

struct Listeners {
    listeners: HashMap<PeerEventType, Vec<Arc<Listener>>>,
}

impl Listeners {
    fn new() -> Self {
        Self {
            listeners: HashMap::new(),
        }
    }

    fn add(&mut self, listener: Arc<Listener>, event_types: Vec<PeerEventType>) {
        for event_type in event_types {
            self.listeners
                .entry(event_type)
                .or_default()
                .push(listener.clone());
        }
    }

    fn get(&self, event_type: &PeerEventType) -> impl Iterator<Item = &Arc<Listener>> {
        self.listeners
            .get(event_type)
            .map(|x| x.iter())
            .unwrap_or_else(|| [].iter())
    }

    fn remove(&mut self, listener_id: &str) {
        for listener_set in self.listeners.values_mut() {
            listener_set.retain(|listener| listener.id != listener_id);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Periodic<T> {
    pub id: T,
    pub period: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Scheduled<T> {
    data: Periodic<T>,
    // the time after we need to run the task
    run_at: Instant,
}

impl<T: Eq> Scheduled<T> {
    fn new(data: Periodic<T>, now: Instant) -> Scheduled<T> {
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

pub struct EventsDispatcher {
    sources: Vec<BoxStream<'static, PeerEvent>>,
    recv_cmd_channel: Inlet<Command>,
    send_events: Outlet<ListenerEvent>,
}

impl EventsDispatcher {
    pub fn new(sources: Vec<BoxStream<'static, PeerEvent>>) -> (Self, EventsDispatcherApi, Inlet<ListenerEvent>) {
        let (send_cmd_channel, recv_cmd_channel) = unbounded();
        let api = EventsDispatcherApi { send_cmd_channel };

        let (send_events, recv_events) = unbounded();

        let this = Self {
            sources: sources,
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

        let mut heap: BinaryHeap<Scheduled<String>> = BinaryHeap::new();
        let mut listeners = Listeners::new();

        loop {
            let now = Instant::now();
            let mut timer = {
                let next_task_in = if let Some(next_task) = heap.peek() {
                    next_task
                        .run_at
                        .checked_duration_since(now)
                        .unwrap_or_default()
                } else {
                    Duration::MAX
                };

                async_std::stream::interval(next_task_in).fuse()
            };

            select! {
                command = recv_cmd_channel.select_next_some() => {
                    match command {
                        Command::Add(listener, config) => {
                            let listener = Arc::new(listener);
                            for config in config.configs {
                                match config {
                                    EventConfig::Timer(config) => {
                                        let periodic = Periodic { id: listener.id.clone(), period: config.period };
                                        heap.push(Scheduled::new(periodic, Instant::now()))
                                    },
                                    EventConfig::PeerEvent(config) => {
                                        listeners.add(listener.clone(), config.events);
                                    },
                                }
                            }
                        },
                        Command::Remove(listener_id, reply) => {
                            heap.retain(|scheduled| scheduled.data.id != listener_id);
                            listeners.remove(&listener_id);
                            if let Err(e) = reply.send(()) {
                                log::warn!("Can't send notification about listener {:?} removal: {:?}", listener_id, e);
                            }
                        },
                    }
                },
                event = sources_channel.select_next_some() => {
                    for listener in listeners.get(&event.get_type()) {
                        if let Err(e) = send_events.unbounded_send(ListenerEvent { id: listener.id.clone(), event: Event::Peer(event.clone()) }) {
                            log::warn!("Can't send notification about event {:?}: {:?}", event, e);
                        }
                    }
                },
                _ = timer.select_next_some() => {
                    let task = heap.pop().expect("billions of years have gone by already?");
                    if let Err(e) = send_events.unbounded_send(ListenerEvent { id: task.data.id.clone(), event: Event::Timer }) {
                        log::warn!("Can't send notification by timer: {:?}", e);
                    }
                    heap.push(task.reschedule(now));
                },

            }
        }
    }
}
#[test]
fn test() {
    let (send1, recv1) = unbounded();
    task::spawn(async move {
        loop {
            task::sleep(std::time::Duration::from_secs(2)).await;
            send1
                .unbounded_send(PeerEvent::Connect {
                    who: "me".to_string(),
                })
                .unwrap();
        }
    });
    let (send2, recv2) = unbounded();
    task::spawn(async move {
        loop {
            task::sleep(std::time::Duration::from_secs(3)).await;
            send2
                .unbounded_send(PeerEvent::Disconnect {
                    who: "me".to_string(),
                })
                .unwrap();
        }
    });

    let cfg = EventsDispatcherCfg {
        sources: vec![recv1, recv2],
    };

    let (dispatcher, api, mut events) = EventsDispatcher::new(cfg);
    dispatcher.start();

    let cfg = ListenerConfig {
        configs: vec![
            EventConfig::PeerEvent(PeerEventConfig {
                events: HashSet::from([PeerEventType::Connect]),
            }),
            EventConfig::Timer(TimerConfig {
                period: Duration::from_secs(1),
            }),
        ],
    };

    api.add(
        Listener {
            id: "1".to_string(),
        },
        cfg,
    )
    .unwrap();

    task::block_on(async move {
        loop {
            select! {
                event = events.select_next_some() => {
                    println!("{:?}", event);
                }
            }
        }
    });
}
