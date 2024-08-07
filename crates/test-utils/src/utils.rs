/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */
use clarity::PrivateKey;
use ivalue_utils::IValue;
use server_config::ChainConfig;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct RetStruct {
    pub ret_code: u32,
    pub error: String,
    pub result: String,
}

pub fn response_to_return(resp: IValue) -> RetStruct {
    match resp {
        IValue::Record(r) => {
            let ret_code = match r.first().unwrap() {
                IValue::U32(u) => *u,
                _ => panic!("unexpected, should be u32 ret_code"),
            };
            let msg = match r.get(1).unwrap() {
                IValue::String(u) => u.to_string(),
                _ => panic!("unexpected, should be string error message"),
            };
            if ret_code == 0 {
                RetStruct {
                    ret_code,
                    result: msg,
                    error: "".to_string(),
                }
            } else {
                RetStruct {
                    ret_code,
                    error: msg,
                    result: "".to_string(),
                }
            }
        }
        _ => panic!("unexpected, should be a record"),
    }
}

pub fn string_result(ret: RetStruct) -> Result<String, String> {
    if ret.ret_code == 0 {
        let hash: String = serde_json::from_str(&ret.result).unwrap();
        Ok(hash)
    } else {
        Err(ret.error)
    }
}

pub fn get_default_chain_config(url: &str) -> ChainConfig {
    ChainConfig {
        http_endpoint: url.to_string(),
        diamond_contract_address: "".to_string(),
        network_id: 0,
        wallet_key: PrivateKey::from_str(
            "0x97a2456e78c4894c62eef6031972d1ca296ed40bf311ab54c231f13db59fc428",
        )
        .unwrap(),
        default_base_fee: None,
        default_priority_fee: None,
    }
}
