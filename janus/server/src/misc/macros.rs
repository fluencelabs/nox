/*
 * Copyright 2019 Fluence Labs Limited
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

// https://github.com/rust-lang/rust/issues/57966#issuecomment-461077932
#[macro_export]
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
