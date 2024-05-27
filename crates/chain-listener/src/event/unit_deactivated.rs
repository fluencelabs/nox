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

use alloy_sol_types::sol;

sol! {
    /// @dev Emitted when a unit deactivated. Unit is deactivated when it moved to deal
    /// @param commitmentId Commitment id
    /// @param unitId Compute unit id which deactivated
    #[derive(Debug)]
    event UnitDeactivated(
        bytes32 indexed commitmentId,
        bytes32 indexed unitId
    );
}

#[cfg(test)]
mod test {
    use crate::event::unit_deactivated::UnitDeactivated;
    use alloy_sol_types::SolEvent;

    use chain_data::{parse_log, Log};

    #[tokio::test]
    async fn test_unit_activated_topic() {
        assert_eq!(
            UnitDeactivated::SIGNATURE_HASH.to_string(),
            "0xbd9cde1bbc961036d34368ae328c38917036a98eacfb025a1ff6d2c6235d0a14"
        );
    }

    #[tokio::test]
    async fn test_chain_parsing_ok() {
        let data = "0x".to_string();

        let topics = vec![
            UnitDeactivated::SIGNATURE_HASH.to_string(),
            "0x91cfcc4a139573b08646960be31b278152ef3480710ab15d9b39262be37038a1".to_string(),
            "0xf3660ca1eaf461cbbb5e1d06ade6ba4a9a503c0d680ba825e09cddd3f9b45fc6".to_string(),
        ];
        let result = parse_log::<UnitDeactivated>(Log {
            data,
            block_number: "".to_string(),
            removed: false,
            topics,
        });

        assert!(result.is_ok(), "can't parse data: {:?}", result);
        let result = result.unwrap();
        assert_eq!(
            result.commitmentId.to_string(),
            "0x91cfcc4a139573b08646960be31b278152ef3480710ab15d9b39262be37038a1" // it's the second topic
        );
        assert_eq!(
            result.unitId.to_string(),
            "0xf3660ca1eaf461cbbb5e1d06ade6ba4a9a503c0d680ba825e09cddd3f9b45fc6" // it's also the third topic
        );
    }
}
