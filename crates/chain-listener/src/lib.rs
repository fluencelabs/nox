#![feature(assert_matches)]
#![feature(try_blocks)]
#![feature(extract_if)]
#![feature(btree_extract_if)]
#![feature(result_option_inspect)]

extern crate core;

pub use ccp::CCPClient;
pub use listener::ChainListener;
pub use listener::ChainListenerConfig;
pub use subscription::WsEventSubscription;
pub use subscription::WsSubscriptionConfig;

mod event;
mod listener;

mod ccp;
mod persistence;
mod subscription;
