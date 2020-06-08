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

// blocks until either SIGINT(Ctrl+C) or SIGTERM signals received
pub fn block_until_ctrlc() {
    let (ctrlc_outlet, ctrlc_inlet) = futures::channel::oneshot::channel();
    let ctrlc_outlet = std::cell::RefCell::new(Some(ctrlc_outlet));

    ctrlc::set_handler(move || {
        println!("ctrlc fired!");
        if let Some(outlet) = ctrlc_outlet.borrow_mut().take() {
            outlet.send(()).expect("sending shutdown signal failed");
        }
    })
    .expect("Error while setting ctrlc handler");

    async_std::task::block_on(ctrlc_inlet).expect("exit oneshot failed");
}
