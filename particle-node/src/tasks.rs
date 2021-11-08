/*
 * Copyright 2020 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_std::task::JoinHandle;
use futures::future::FusedFuture;
use futures::FutureExt;

/// Holds handles to spawned tasks
pub struct Tasks {
    name: &'static str,
    /// Task that processes particles from particle stream
    pub tasks: Vec<JoinHandle<()>>,
}

impl Tasks {
    pub fn new(name: &'static str, tasks: Vec<JoinHandle<()>>) -> Self {
        Self { name, tasks }
    }

    pub async fn cancel(self) {
        for task in self.tasks {
            task.cancel().await;
        }
    }
}

impl Future for Tasks {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.tasks
            .drain_filter(|mut task| task.poll_unpin(cx).is_ready());

        if self.is_terminated() {
            log::warn!("{} tasks terminated", self.name);
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

impl FusedFuture for Tasks {
    fn is_terminated(&self) -> bool {
        self.tasks.is_empty()
    }
}
