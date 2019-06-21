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

package fluence.effects.tendermint.rpc

object TestData {

  val block: String =
    """
      |{
      | "jsonrpc": "2.0",
      | "id": "1#event",
      | "result": {
      |  "query": "tm.event = 'NewBlock'",
      |  "data": {
      |   "type": "tendermint/event/NewBlock",
      |   "value": {
      |    "block": {
      |     "header": {
      |      "version": {
      |       "block": "10",
      |       "app": "0"
      |      },
      |      "chain_id": "18",
      |      "height": "11",
      |      "time": "2019-05-23T10:19:27.569664571Z",
      |      "num_txs": "2",
      |      "total_txs": "9",
      |      "last_block_id": {
      |       "hash": "4C1BDE177A4E5F8F0016DFD4B4AACA96CD58FEC96D542532A38840E394C88674",
      |       "parts": {
      |        "total": "1",
      |        "hash": "CDB693416AD81446A0BF916B90B4D42CD871E686EFBEC6CD467F83840EB87401"
      |       }
      |      },
      |      "last_commit_hash": "E0DD88ABB4DC7F294D246950D3B93B71678371E9277ADFA5D872EB52E23C51BC",
      |      "data_hash": "0BA0E5C694A72913DAC5DC5874E6BED473B0F6052798FB58E96BE3A0F28EFB4E",
      |      "validators_hash": "E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F475374",
      |      "next_validators_hash": "E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F475374",
      |      "consensus_hash": "048091BC7DDC283F77BFBF91D73C44DA58C3DF8A9CBC867405D8B7F3DAADA22F",
      |      "app_hash": "5DF6E0E2761359D30A8275058E299FCC0381534545F55CF43E41983F5D4C9456",
      |      "last_results_hash": "34661E25AC25E94AD1B869A3DF67F2D008B01E440382A6BD6903697703DA3F53",
      |      "evidence_hash": "",
      |      "proposer_address": "991C9F03698AC07BEB41B71A87715FC4364A994A"
      |     },
      |     "data": {
      |      "txs": [
      |       "YXdCNk9JeTQxSHh5LzEKMGRmNWVkOWFlNmFmNzQ4NjMzYzRmM2UxNDkzNjM1ZTBiOGM5ZjRkN2I5NjUwNzEyYjM2MGY2MmU0ZDkzZDcyNTVjODI1ZGM1MGVmZGI2MWQ1OTA1YmYxNTcxZGQ0ZTIxMGQxMTg5YWNmNWVmNTUxYTRjNzgzYzcyM2NkZGY1YWIKMQpJTlNFUlQgSU5UTyB1c2VycyBWQUxVRVMoMSwgJ1NhcmEnLCAyMyksICgyLCAnQm9iJywgMTkpLCAoMywgJ0Nhcm9saW5lJywgMzEpLCAoNCwgJ01heCcsIDI3KQ==",
      |       "YXdCNk9JeTQxSHh5LzAKODAwYjc1OGE3ZWMzMTllMTU0M2UxODY1YzIwNjcyMmFjNjgyZGMyNWQ4NmNmYWMxZDUxNTM0MTkyYTAzZGFmNTUwNmNmYTJjNDQ3ZGU1OTczMDg3ODE0Y2I1YWM0MDk1ZTBkOTdmNzcwMDdkNmYzMWVkOWFmZTA5MzMzMTFjZmEKMApDUkVBVEUgVEFCTEUgdXNlcnMoaWQgaW50LCBuYW1lIHZhcmNoYXIoMTI4KSwgYWdlIGludCk="
      |      ]
      |     },
      |     "evidence": {
      |      "evidence": null
      |     },
      |     "last_commit": {
      |      "block_id": {
      |       "hash": "4C1BDE177A4E5F8F0016DFD4B4AACA96CD58FEC96D542532A38840E394C88674",
      |       "parts": {
      |        "total": "1",
      |        "hash": "CDB693416AD81446A0BF916B90B4D42CD871E686EFBEC6CD467F83840EB87401"
      |       }
      |      },
      |      "precommits": [
      |       {
      |        "type": 2,
      |        "height": "10",
      |        "round": "0",
      |        "block_id": {
      |         "hash": "4C1BDE177A4E5F8F0016DFD4B4AACA96CD58FEC96D542532A38840E394C88674",
      |         "parts": {
      |          "total": "1",
      |          "hash": "CDB693416AD81446A0BF916B90B4D42CD871E686EFBEC6CD467F83840EB87401"
      |         }
      |        },
      |        "timestamp": "2019-05-23T10:19:27.571292268Z",
      |        "validator_address": "04C60B72246943675E2F3AADA00E30EC41AA7D4E",
      |        "validator_index": "0",
      |        "signature": "1+xaBh7zgsTmHDnOS5I2Y4G6XRo94bDZ3pe4VEP78YwczfzEJ6rTjDjSl4AO9lDCUz0PJiBUW8iZuI8BjYMgAg=="
      |       },
      |       {
      |        "type": 2,
      |        "height": "10",
      |        "round": "0",
      |        "block_id": {
      |         "hash": "4C1BDE177A4E5F8F0016DFD4B4AACA96CD58FEC96D542532A38840E394C88674",
      |         "parts": {
      |          "total": "1",
      |          "hash": "CDB693416AD81446A0BF916B90B4D42CD871E686EFBEC6CD467F83840EB87401"
      |         }
      |        },
      |        "timestamp": "2019-05-23T10:19:27.603795009Z",
      |        "validator_address": "0D71FECA786E7FE982E6FC13422AAC82329DF077",
      |        "validator_index": "1",
      |        "signature": "1heQoqNe4rW26FslBcj0JWiHkEoSgEfJ0Sm5h9GinSwNvSIN7PAwG6oDHXyTFOLwFTHA54xOyHRGSbhwCFwyAQ=="
      |       },
      |       {
      |        "type": 2,
      |        "height": "10",
      |        "round": "0",
      |        "block_id": {
      |         "hash": "4C1BDE177A4E5F8F0016DFD4B4AACA96CD58FEC96D542532A38840E394C88674",
      |         "parts": {
      |          "total": "1",
      |          "hash": "CDB693416AD81446A0BF916B90B4D42CD871E686EFBEC6CD467F83840EB87401"
      |         }
      |        },
      |        "timestamp": "2019-05-23T10:19:27.502300199Z",
      |        "validator_address": "991C9F03698AC07BEB41B71A87715FC4364A994A",
      |        "validator_index": "2",
      |        "signature": "anGoUg50iwb3Z2Ugb74ME9PfnRd/vdL1xSkZPJvvu7nNHpYWuS+98lF3rXcGsvn29nFNYFwda1wpgjgncuSdCw=="
      |       },
      |       {
      |        "type": 2,
      |        "height": "10",
      |        "round": "0",
      |        "block_id": {
      |         "hash": "4C1BDE177A4E5F8F0016DFD4B4AACA96CD58FEC96D542532A38840E394C88674",
      |         "parts": {
      |          "total": "1",
      |          "hash": "CDB693416AD81446A0BF916B90B4D42CD871E686EFBEC6CD467F83840EB87401"
      |         }
      |        },
      |        "timestamp": "2019-05-23T10:19:27.569664571Z",
      |        "validator_address": "9F16F63227F11942E6E4A3282B2A293E4BF8206C",
      |        "validator_index": "3",
      |        "signature": "NVajbRq+aGVX17vXAK/zksmdm/9ODW6UtdutYHtngm6+dk4lt1KTKqDwwLNNiWlgod43CijQcKMURFJQifuMAQ=="
      |       }
      |      ]
      |     }
      |    },
      |    "result_begin_block": {},
      |    "result_end_block": {
      |     "validator_updates": null
      |    }
      |   }
      |  }
      | }
      |}
    """.stripMargin
}
