//! Rust SDK for writing applications for Fluence.
#![doc(html_root_url = "https://docs.rs/fluence_sdk/0.0.5")]
#![feature(allocator_api)]

extern crate core;

pub mod memory;

#[cfg(feature = "wasm_logger")]
pub mod logger;
