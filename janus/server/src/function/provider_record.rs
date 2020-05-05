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

use crate::function::router::Service;
use libp2p::kad::Record;
use std::error::Error;

pub fn provider_to_record(service_id: String, service: &Service) -> Record {
    let bytes = service_to_bytes(service);
    Record::new(service_id.as_bytes().to_vec(), bytes)
}

pub fn record_to_provider(record: &Record) -> Result<Service, Box<dyn Error>> {
    service_from_bytes(record.value.as_slice())
}

fn service_to_bytes(service: &Service) -> Vec<u8> {
    serde_json::to_vec(service).expect("service should be serialized to bytes")
}

fn service_from_bytes(bytes: &[u8]) -> Result<Service, Box<dyn Error>> {
    serde_json::from_slice(bytes).map_err(|e| e.into())
}
