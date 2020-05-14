/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
 */

#[macro_export] // https://github.com/rust-lang/rust/issues/57966#issuecomment-461077932
/// Intended to simplify simple polling functions that just return internal events from a
/// internal queue.
macro_rules! event_polling {
    ($func_name:ident, $event_field_name:ident, $poll_type:ty) => {
        fn $func_name(
            &mut self,
            _: &mut std::task::Context,
            _: &mut impl libp2p::swarm::PollParameters,
        ) -> std::task::Poll<$poll_type> {
            use std::task::Poll;

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
