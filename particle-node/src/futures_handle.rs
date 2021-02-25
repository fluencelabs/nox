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

use async_std::task::JoinHandle;
use futures::future::{BoxFuture, FusedFuture};
use futures::stream::Fuse;
use futures::{FutureExt, Stream};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct FuturesHandle {
    pub particles: Option<JoinHandle<()>>,
    pub reconnect_bootstraps: Option<JoinHandle<()>>,
    pub run_bootstrap: Option<JoinHandle<()>>,
}

impl FuturesHandle {
    pub fn new(
        particles: JoinHandle<()>,
        reconnect_bootstraps: JoinHandle<()>,
        run_bootstrap: JoinHandle<()>,
    ) -> Self {
        Self {
            particles: Some(particles),
            reconnect_bootstraps: Some(reconnect_bootstraps),
            run_bootstrap: Some(run_bootstrap),
        }
    }

    pub async fn cancel(mut self) {
        if let Some(run_bootstrap) = self.run_bootstrap {
            run_bootstrap.cancel().await;
        };
        if let Some(reconnect_bootstraps) = self.reconnect_bootstraps {
            reconnect_bootstraps.cancel().await;
        };
        if let Some(particles) = self.particles {
            particles.cancel().await;
        };
    }
}

impl Future for FuturesHandle {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut p_ready = false;
        if let Some(particles) = self.particles.as_mut() {
            if particles.poll_unpin(cx).is_ready() {
                p_ready = true;
            }
        }
        if p_ready {
            self.particles.take();
        }

        let mut r_ready = false;
        if let Some(reconnect_bootstraps) = self.reconnect_bootstraps.as_mut() {
            if reconnect_bootstraps.poll_unpin(cx).is_ready() {
                r_ready = true;
            }
        }
        if r_ready {
            self.reconnect_bootstraps.take();
        }

        let mut b_ready = false;
        if let Some(run_bootstrap) = self.run_bootstrap.as_mut() {
            if run_bootstrap.poll_unpin(cx).is_ready() {
                b_ready = true;
            }
        }
        if b_ready {
            self.run_bootstrap.take();
        }

        if self.is_terminated() {
            log::warn!("FuturesHandle terminated");
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

impl FusedFuture for FuturesHandle {
    fn is_terminated(&self) -> bool {
        self.particles.is_none()
            && self.reconnect_bootstraps.is_none()
            && self.run_bootstrap.is_none()
    }
}
