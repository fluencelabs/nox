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

// blocks until either SIGINT(Ctrl+C) or SIGTERM signals received
pub fn block_until_ctrlc() {
    let (ctrlc_outlet, ctrlc_inlet) = futures::channel::oneshot::channel();
    let ctrlc_outlet = std::cell::RefCell::new(Some(ctrlc_outlet));

    ctrlc::set_handler(move || {
        if let Some(outlet) = ctrlc_outlet.borrow_mut().take() {
            outlet.send(()).expect("sending shutdown signal failed");
        }
    })
    .expect("Error while setting ctrlc handler");

    async_std::task::block_on(ctrlc_inlet).expect("exit oneshot failed");
}
