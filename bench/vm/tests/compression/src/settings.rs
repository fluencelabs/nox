/*
 * Copyright 2018 Fluence Labs Limited
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
/// This value is used as the first seed of pseudo-random generator for sequence generation.
pub const SEED: &str = env!("SEED");

/// Count of compression iterations.
pub const ITERATIONS_COUNT: &str = env!("ITERATIONS_COUNT");

/// Size of sequence that should be compressed on each iteration.
pub const SEQUENCE_SIZE: &str = env!("SEQUENCE_SIZE");
