use std::future::Future;

use enum_dispatch::enum_dispatch;
use libp2p::PeerId;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

#[enum_dispatch]
pub(crate) trait SpawnFunctions {
    fn spawn_function_call<F>(&self, function_identity: String, fut: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static;
    fn spawn_avm_call<F, R>(&self, fut: F) -> JoinHandle<F::Output>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static;
}

#[derive(Clone)]
#[enum_dispatch(SpawnFunctions)]
pub enum Spawner {
    Root(RootSpawner),
    Worker(WorkerSpawner),
}

#[derive(Clone)]
pub struct RootSpawner {
    runtime_handle: Handle,
}

impl RootSpawner {
    pub(crate) fn new(runtime_handle: Handle) -> Self {
        Self { runtime_handle }
    }
}
impl SpawnFunctions for RootSpawner {
    fn spawn_function_call<F: Future>(
        &self,
        function_identity: String,
        fut: F,
    ) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let task_name = format!("Call function root:{}", &function_identity);
        let builder = tokio::task::Builder::new().name(task_name.as_str());

        let handle = self.runtime_handle.clone();
        builder
            .spawn_blocking_on(move || handle.block_on(fut), &self.runtime_handle)
            .expect("Failed to spawn a task")
    }

    fn spawn_avm_call<F, R>(&self, fut: F) -> JoinHandle<F::Output>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        self.runtime_handle.spawn_blocking(fut)
    }
}

#[derive(Clone)]
pub struct WorkerSpawner {
    worker_id: PeerId,
    runtime_handle: Handle,
}

impl WorkerSpawner {
    pub(crate) fn new(runtime_handle: Handle, worker_id: PeerId) -> Self {
        Self {
            runtime_handle,
            worker_id,
        }
    }
}

impl SpawnFunctions for WorkerSpawner {
    fn spawn_function_call<F: Future>(
        &self,
        function_identity: String,
        fut: F,
    ) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let task_name = format!("Call function {}:{}", self.worker_id, &function_identity);

        let builder = tokio::task::Builder::new().name(task_name.as_str());

        builder
            .spawn_on(fut, &self.runtime_handle)
            .expect("Failed to spawn a task")
    }

    fn spawn_avm_call<F, R>(&self, fut: F) -> JoinHandle<F::Output>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        self.runtime_handle.spawn(async { fut() })
    }
}
