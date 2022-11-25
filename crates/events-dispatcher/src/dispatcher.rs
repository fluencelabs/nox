use crate::api::*;
use async_std::sync::Arc;
use async_std::task;
use fluence_libp2p::types::Inlet;
use futures::stream;
use futures::{channel::mpsc::unbounded, select, StreamExt};
use std::collections::{HashMap, HashSet};

struct Listeners {
    listeners: HashMap<EventType, Vec<Arc<Listener<Event>>>>,
}

impl Listeners {
    fn new() -> Self {
        Self {
            listeners: HashMap::new(),
        }
    }

    fn add(&mut self, listener: Listener<Event>, event_types: HashSet<EventType>) {
        let listener = Arc::new(listener);
        for event_type in event_types {
            self.listeners
                .entry(event_type)
                .or_default()
                .push(listener.clone());
        }
    }

    fn get(&self, event_type: &EventType) -> impl Iterator<Item = &Arc<Listener<Event>>> {
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

pub struct EventsDispatcherCfg {
    pub sources: Vec<Inlet<Event>>,
}

pub struct EventsDispatcher {
    sources: Vec<Inlet<Event>>,
    api_channel: Inlet<Command>,
}

impl EventsDispatcher {
    pub fn new(cfg: EventsDispatcherCfg) -> (Self, EventsDispatcherApi) {
        let (send_channel, recv) = unbounded();
        let api = EventsDispatcherApi { send_channel };
        let this = Self {
            sources: cfg.sources,
            api_channel: recv,
        };
        (this, api)
    }

    pub fn start(self) -> task::JoinHandle<()> {
        task::spawn(self.run())
    }

    async fn run(self) {
        let mut api_channel = self.api_channel.fuse();
        let sources = self
            .sources
            .into_iter()
            .map(|source| source.fuse())
            .collect::<Vec<_>>();
        let mut sources_channel = stream::select_all(sources);
        let mut listeners = Listeners::new();
        loop {
            select! {
                command = api_channel.select_next_some() => {
                    match command {
                        Command::Add(listener, config) => {
                            listeners.add(listener, config.events);
                        },
                        Command::Remove(listener_id) => {
                            listeners.remove(&listener_id);
                        },
                    }
                },
                event = sources_channel.select_next_some() => {
                    for listener in listeners.get(&event.get_type()) {
                        listener.notify(&event);
                    }
                }
            };
        }
    }
}
