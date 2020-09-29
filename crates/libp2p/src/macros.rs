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

#[macro_export] // https://github.com/rust-lang/rust/issues/57966#issuecomment-461077932
/// Intended to simplify simple polling functions that just return internal events from a
/// internal queue.
macro_rules! event_polling {
    ($func_name:ident, $event_field_name:ident, $poll_type:ty$(, $tick:ident)?) => {
        fn $func_name(
            &mut self,
            _: &mut std::task::Context,
            _: &mut impl libp2p::swarm::PollParameters,
        ) -> std::task::Poll<$poll_type> {
            use std::task::Poll;

            $(self.$tick())?;

            if let Some(event) = self.$event_field_name.pop_front() {
                return Poll::Ready(event);
            }

            Poll::Pending
        }
    };
}

#[macro_export] // https://github.com/rust-lang/rust/issues/57966#issuecomment-461077932
/// Generates a type of events produced by Swarm by its name
macro_rules! generate_swarm_event_type {
    ($swarm_type_name:ty) => {
        ::libp2p::swarm::NetworkBehaviourAction<
                <<<$swarm_type_name as ::libp2p::swarm::NetworkBehaviour>::ProtocolsHandler
                    as ::libp2p::swarm::IntoProtocolsHandler>::Handler
                    as ::libp2p::swarm::protocols_handler::ProtocolsHandler>::InEvent, // InEvent
                <$swarm_type_name as ::libp2p::swarm::NetworkBehaviour>::OutEvent // OutEvent
            >
    }
}

#[macro_export]
macro_rules! poll_loop {
    ($self:expr,$behaviour:expr,$cx:expr,$params:expr$(,$into_event:expr)?) => {{
        loop {
            match NetworkBehaviour::poll($behaviour, $cx, $params) {
                Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::GenerateEvent(event)) => {
                    ::libp2p::swarm::NetworkBehaviourEventProcess::inject_event($self, event)
                }
                Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::NotifyHandler {
                    peer_id,
                    event,
                    handler,
                }) => {
                    $(let event = $into_event(event);)?
                    return Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::NotifyHandler {
                        peer_id,
                        event,
                        handler,
                    })
                }
                Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::DialAddress { address }) => {
                    return Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::DialAddress { address })
                }
                Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::ReportObservedAddr { address }) => {
                    return Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::ReportObservedAddr { address })
                }
                Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::DialPeer { peer_id, condition }) => {
                    return Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::DialPeer { peer_id, condition })
                }
                Poll::Pending => break,
            }
        }
    }};
}
