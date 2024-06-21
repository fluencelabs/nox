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
                <<<$swarm_type_name as ::libp2p::swarm::NetworkBehaviour>::ConnectionHandler
                    as ::libp2p::swarm::IntoConnectionHandler>::Handler
                    as ::libp2p::swarm::protocols_handler::ConnectionHandler>::InEvent, // InEvent
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
                Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::ReportObservedAddr { address, score }) => {
                    return Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::ReportObservedAddr { address, score })
                }
                Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::DialPeer { peer_id, condition }) => {
                    return Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::DialPeer { peer_id, condition })
                }
                Poll::Pending => break,
            }
        }
    }};
}
