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
/// this seed is used for deterministic operation count on different launches
pub const SEED: &str = env!("SEED");

/// a matrix size
pub const MATRIX_SIZE: &str = env!("MATRIX_SIZE");

/// count of test iterations
pub const ITERATIONS_COUNT: &str = env!("ITERATIONS_COUNT");

/// 1117 due to prevent overflow in matrix multiplication
pub const GENERATION_INTERVAL: f64 = 1117.0;

pub extern crate nalgebra;
use nalgebra::DMatrix;
/// exactly matrix type
pub type Matrix = DMatrix<f64>;
