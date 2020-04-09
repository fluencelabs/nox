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

use futures::channel::{mpsc, oneshot};

mod enqueue_result;
pub mod macros;
mod peer_id;
mod promise_queue;
mod safe_multihash;
mod waiting_queue;

/// An input port of actor (aka Akka Inlet).
/// Could be used to send MsgType messages in the actor.
pub type Inlet<MsgType> = mpsc::UnboundedReceiver<MsgType>;

/// An output port of actor (aka Akka Outlet).
/// Could be used to receive MsgType messages from the actor.
pub type Outlet<MsgType> = mpsc::UnboundedSender<MsgType>;

/// An oneshot input port of actor (aka Akka Inlet).
/// Could be used to gracefully shutting down of the actor.
pub type OneshotInlet<MsgType> = oneshot::Receiver<MsgType>;

/// An oneshot output port of actor (aka Akka Outlet).
/// Could be used to gracefully shutting down of the actor.
pub type OneshotOutlet<MsgType> = oneshot::Sender<MsgType>;

pub use peer_id::{multihash_serializer, peerid_serializer, provider_serializer};
pub use promise_queue::PromiseQueue;
pub use safe_multihash::SafeMultihash;
pub use waiting_queue::WaitingQueue;
