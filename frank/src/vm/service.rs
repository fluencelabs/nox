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

use crate::vm::errors::FrankError;
use crate::vm::frank_result::FrankResult;
use sha2::digest::generic_array::GenericArray;

/// Describes a service behaviour in the Fluence network.
pub trait FluenceService {
    /// Invokes a module supplying byte array and expecting byte array with some outcome back.
    fn invoke(&mut self, fn_argument: &[u8]) -> Result<FrankResult, FrankError>;

    /// Computes hash of the internal modules state.
    fn compute_state_hash(
        &mut self,
    ) -> GenericArray<u8, <sha2::Sha256 as sha2::digest::FixedOutput>::OutputSize>;
}
