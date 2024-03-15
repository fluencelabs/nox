use std::sync::Arc;

use futures::StreamExt;
use tokio::sync::mpsc::Receiver;
use tokio_stream::wrappers::ReceiverStream;

use crate::errors::PersistError;
use crate::CoreManager;

pub trait PersistentCoreManagerFunctions {
    fn persist(&self) -> Result<(), PersistError>;
}

pub struct PersistenceTask {
    receiver: Receiver<()>,
}

impl PersistenceTask {
    pub(crate) fn new(receiver: Receiver<()>) -> Self {
        Self { receiver }
    }
}

impl PersistenceTask {
    async fn process_events<Src>(stream: Src, core_manager: Arc<CoreManager>)
    where
        Src: futures::Stream<Item = ()> + Unpin + Send + Sync + 'static,
    {
        let core_manager = core_manager.clone();
        // We are not interested in the content of the event
        // We are waiting for the event to initiate the persistence process
        stream.for_each(move |_| {
            let core_manager = core_manager.clone();
            async move {
                tokio::task::spawn_blocking(move || {
                    if let CoreManager::Persistent(manager) = core_manager.as_ref() {
                        let result = manager.persist();
                        match result {
                            Ok(_) => {
                                tracing::debug!(target: "core-manager", "Core state was persisted");
                            }
                            Err(err) => {
                                tracing::warn!(target: "core-manager", "Failed to save core state {err}");
                            }
                        }
                    }
                })
                    .await
                    .expect("Could not spawn persist task")
            }
        }).await;
    }

    pub async fn run(self, core_manager: Arc<CoreManager>) {
        let stream = ReceiverStream::from(self.receiver);

        tokio::task::Builder::new()
            .name("core-manager-persist")
            .spawn(Self::process_events(stream, core_manager))
            .expect("Could not spawn persist task");
    }
}
