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
use futures::FutureExt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct FuturesHandle {
    pub particles: JoinHandle<()>,
    pub reconnect_bootstraps: JoinHandle<()>,
    pub run_bootstrap: JoinHandle<()>,
}

impl FuturesHandle {
    pub async fn cancel(mut self) {
        self.run_bootstrap.cancel().await;
        self.reconnect_bootstraps.cancel().await;
        self.particles.cancel().await;
    }
}

impl Future for FuturesHandle {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut p_ready = false;

        while self.particles.poll_unpin(cx).is_ready() {
            p_ready = true;
        }

        let mut r_ready = false;
        while self.reconnect_bootstraps.poll_unpin(cx).is_ready() {
            r_ready = true;
        }

        let mut b_ready = false;
        while self.run_bootstrap.poll_unpin(cx).is_ready() {
            b_ready = true;
        }

        if p_ready || r_ready || b_ready {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
