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

use crate::node_service::relay::RelayEvent;

// TODO: get rid of InnerMessage by augmenting relay event
#[derive(Debug)]
pub enum InnerMessage {
    Relay(RelayEvent),
    // This is needed because UpgradeOutbound states OutType = ()
    Sent,
}

impl From<RelayEvent> for InnerMessage {
    #[inline]
    fn from(event: RelayEvent) -> Self {
        InnerMessage::Relay(event)
    }
}

// TODO: Why, where is it needed? Is it really "sent", or has some other meaning? Wtf? :(
impl From<()> for InnerMessage {
    #[inline]
    fn from(_: ()) -> Self {
        InnerMessage::Sent
    }
}
