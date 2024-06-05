/*
 * Copyright 2024 Fluence DAO
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

#![feature(assert_matches)]
#![feature(result_flattening)]
#![feature(try_blocks)]
#![feature(iter_array_chunks)]

mod connector;
mod error;
mod function;

mod types;

pub use connector::CCInitParams;
pub use connector::ChainConnector;
pub use connector::HttpChainConnector;
pub use error::ConnectorError;
pub use function::*;
