/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::future::Future;

use enum_dispatch::enum_dispatch;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use tokio_util::context::TokioContext;
use workers::WorkerId;

/// The `SpawnFunctions` trait defines methods for spawning asynchronous tasks with different configurations.
///
/// It provides three methods:
/// - `spawn_function_call`: Spawns a task representing a function call.
/// - `spawn_avm_call`: Spawns a task representing an AVM call.
/// - `spawn_io`: Spawns a task for asynchronous I/O operations.
///
/// Implementations should handle the specifics of spawning tasks based on the provided context.
#[enum_dispatch]
pub(crate) trait SpawnFunctions {
    /// Spawns a task representing a function call.
    ///
    /// # Parameters
    ///
    /// - `function_identity`: A string identifier for the function.
    /// - `fut`: The future representing the asynchronous function call.
    ///
    /// # Returns
    ///
    /// A `JoinHandle` representing the handle to the spawned task.
    ///
    /// # Type Parameters
    ///
    /// - `F`: The type of the future representing the asynchronous function call.
    fn spawn_function_call<F>(&self, function_identity: String, fut: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static;

    /// Spawns a task representing an AVM call.
    ///
    /// # Parameters
    ///
    /// - `func`: The closure returning a result of type `R` when executed asynchronously.
    ///
    /// # Returns
    ///
    /// A `JoinHandle` representing the handle to the spawned task.
    ///
    /// # Type Parameters
    ///
    /// - `F`: The type of the closure.
    /// - `R`: The type of the result returned by the closure.
    fn spawn_avm_call<F>(&self, fut: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static;

    /// Shift execution to the specific pool
    ///
    /// # Parameters
    ///
    /// - `fut`: The future representing the asynchronous I/O task.
    ///
    /// # Returns
    ///
    /// A `TokioContext` wrapping the future and associated with a specific runtime handle.
    ///
    /// # Type Parameters
    ///
    /// - `F`: The type of the future representing the asynchronous task.
    fn wrap<F>(&self, fut: F) -> TokioContext<F>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static;
}

/// The `Spawner` enum represents a spawner that can be either a `RootSpawner` or a `WorkerSpawner`.
///
/// It uses the `SpawnFunctions` trait to provide a common interface for spawning asynchronous tasks
/// with different configurations, such as spawning on the root runtime or worker runtime.
#[derive(Clone)]
#[enum_dispatch(SpawnFunctions)]
pub enum Spawner {
    /// Represents a spawner for the root runtime.
    Root(RootSpawner),

    /// Represents a spawner for a worker runtime associated with a specific worker ID.
    Worker(WorkerSpawner),
}

/// The `RootSpawner` struct represents a spawner for the root runtime.
///
/// It implements the `SpawnFunctions` trait to provide methods for spawning asynchronous tasks
/// on the root runtime with specific configurations.
#[derive(Clone)]
pub struct RootSpawner {
    runtime_handle: Handle,
}

impl RootSpawner {
    /// Creates a new `RootSpawner` instance with the given runtime handle.
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

    fn spawn_avm_call<F>(&self, fut: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime_handle
            .spawn_blocking(|| Handle::current().block_on(fut))
    }

    fn wrap<F>(&self, fut: F) -> TokioContext<F>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        TokioContext::new(fut, self.runtime_handle.clone())
    }
}

/// The `WorkerSpawner` struct represents a spawner for a worker runtime associated with a specific worker ID.
///
/// It implements the `SpawnFunctions` trait to provide methods for spawning asynchronous tasks
/// on the worker runtime with specific configurations.
#[derive(Clone)]
pub struct WorkerSpawner {
    worker_id: WorkerId,
    runtime_handle: Handle,
}

impl WorkerSpawner {
    /// Creates a new `WorkerSpawner` instance with the given runtime handle and worker ID.
    pub(crate) fn new(runtime_handle: Handle, worker_id: WorkerId) -> Self {
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

    fn spawn_avm_call<F>(&self, fut: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime_handle.spawn(fut)
    }

    fn wrap<F>(&self, fut: F) -> TokioContext<F>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        TokioContext::new(fut, self.runtime_handle.clone())
    }
}
