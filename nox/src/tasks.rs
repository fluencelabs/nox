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
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::future::FusedFuture;
use futures::FutureExt;
use tokio::task::JoinHandle;

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
            task.abort();
        }
    }
}

impl Future for Tasks {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let _ = self.tasks.extract_if(|task| task.poll_unpin(cx).is_ready());

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
