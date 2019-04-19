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

package fluence.effects.tendermint.block

object TestData {

  val blockResponse =
    """
      |{
      |  "jsonrpc": "2.0",
      |  "id": "dontcare",
      |  "result": {
      |    "block_meta": {
      |      "block_id": {
      |        "hash": "C921CCB37C268965A56FC546713419AEF683201D5151613E07CBD9293308027F",
      |        "parts": {
      |          "total": "1",
      |          "hash": "046C3623869234B711759E66664CC5728B16F577C8D3FFE436278C4D8075E635"
      |        }
      |      },
      |      "header": {
      |        "version": {
      |          "block": "10",
      |          "app": "0"
      |        },
      |        "chain_id": "10",
      |        "height": "17",
      |        "time": "2019-04-17T13:30:03.536359799Z",
      |        "num_txs": "4",
      |        "total_txs": "58",
      |        "last_block_id": {
      |          "hash": "1E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188",
      |          "parts": {
      |            "total": "1",
      |            "hash": "D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B785"
      |          }
      |        },
      |        "last_commit_hash": "4B2FE08AE3C3F7573DB0A36250FB2AABCB1893638EBBECAB7356DDCEF358B423",
      |        "data_hash": "61135E296D5570C607C456416CBD6827E21DED57F7FE2F08879ADCC8228FA7C0",
      |        "validators_hash": "E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F475374",
      |        "next_validators_hash": "E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F475374",
      |        "consensus_hash": "048091BC7DDC283F77BFBF91D73C44DA58C3DF8A9CBC867405D8B7F3DAADA22F",
      |        "app_hash": "DA265DD0F482C2D76B7E67B0128B391FED2971CD74E6DED3EDCCBAB200653DAE",
      |        "last_results_hash": "7FD69E789D2A445E0F3258965481A2E70D5EB9960BCAF065B08C417EF045A846",
      |        "evidence_hash": "",
      |        "proposer_address": "04C60B72246943675E2F3AADA00E30EC41AA7D4E"
      |      }
      |    },
      |    "block": {
      |      "header": {
      |        "version": {
      |          "block": "10",
      |          "app": "0"
      |        },
      |        "chain_id": "10",
      |        "height": "17",
      |        "time": "2019-04-17T13:30:03.536359799Z",
      |        "num_txs": "4",
      |        "total_txs": "58",
      |        "last_block_id": {
      |          "hash": "1E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188",
      |          "parts": {
      |            "total": "1",
      |            "hash": "D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B785"
      |          }
      |        },
      |        "last_commit_hash": "4B2FE08AE3C3F7573DB0A36250FB2AABCB1893638EBBECAB7356DDCEF358B423",
      |        "data_hash": "61135E296D5570C607C456416CBD6827E21DED57F7FE2F08879ADCC8228FA7C0",
      |        "validators_hash": "E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F475374",
      |        "next_validators_hash": "E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F475374",
      |        "consensus_hash": "048091BC7DDC283F77BFBF91D73C44DA58C3DF8A9CBC867405D8B7F3DAADA22F",
      |        "app_hash": "DA265DD0F482C2D76B7E67B0128B391FED2971CD74E6DED3EDCCBAB200653DAE",
      |        "last_results_hash": "7FD69E789D2A445E0F3258965481A2E70D5EB9960BCAF065B08C417EF045A846",
      |        "evidence_hash": "",
      |        "proposer_address": "04C60B72246943675E2F3AADA00E30EC41AA7D4E"
      |      },
      |      "data": {
      |        "txs": [
      |          "WGVTUTFSZG9Ua3dvLzU0CkNSRUFURSBUQUJMRSB1c2VycyhpZCBpbnQsIG5hbWUgdmFyY2hhcigxMjgpLCBhZ2UgaW50KQ==",
      |          "WGVTUTFSZG9Ua3dvLzU2CkNSRUFURSBUQUJMRSB1c2VycyhpZCBpbnQsIG5hbWUgdmFyY2hhcigxMjgpLCBhZ2UgaW50KQ==",
      |          "WGVTUTFSZG9Ua3dvLzU1CklOU0VSVCBJTlRPIHVzZXJzIFZBTFVFUygxLCAnU2FyYScsIDIzKQ==",
      |          "WGVTUTFSZG9Ua3dvLzU3CklOU0VSVCBJTlRPIHVzZXJzIFZBTFVFUygxLCAnU2FyYScsIDIzKQ=="
      |        ]
      |      },
      |      "evidence": {
      |        "evidence": null
      |      },
      |      "last_commit": {
      |        "block_id": {
      |          "hash": "1E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188",
      |          "parts": {
      |            "total": "1",
      |            "hash": "D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B785"
      |          }
      |        },
      |        "precommits": [
      |          {
      |            "type": 2,
      |            "height": "16",
      |            "round": "0",
      |            "block_id": {
      |              "hash": "1E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188",
      |              "parts": {
      |                "total": "1",
      |                "hash": "D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B785"
      |              }
      |            },
      |            "timestamp": "2019-04-17T13:30:03.536359799Z",
      |            "validator_address": "04C60B72246943675E2F3AADA00E30EC41AA7D4E",
      |            "validator_index": "0",
      |            "signature": "Z09xcrfz9T6+3q1Yk+gxUo2todPI7mebKed6zO+i1pnIMPdFbSFT9JJjxo5J9HLrn4x2Fqf3QYefQ8lQGNMzBg=="
      |          },
      |          null,
      |          {
      |            "type": 2,
      |            "height": "16",
      |            "round": "0",
      |            "block_id": {
      |              "hash": "1E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188",
      |              "parts": {
      |                "total": "1",
      |                "hash": "D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B785"
      |              }
      |            },
      |            "timestamp": "2019-04-17T13:30:03.536359799Z",
      |            "validator_address": "991C9F03698AC07BEB41B71A87715FC4364A994A",
      |            "validator_index": "2",
      |            "signature": "VkQicfjxbG+EsHimIXr87a7w8KkHnAq/l60Cv+0oY+rthLIw77NpNhjsMRXVBTiMJzZ3abTBvBUb9jrwPClSCA=="
      |          },
      |          {
      |            "type": 2,
      |            "height": "16",
      |            "round": "0",
      |            "block_id": {
      |              "hash": "1E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188",
      |              "parts": {
      |                "total": "1",
      |                "hash": "D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B785"
      |              }
      |            },
      |            "timestamp": "2019-04-17T13:30:03.536359799Z",
      |            "validator_address": "9F16F63227F11942E6E4A3282B2A293E4BF8206C",
      |            "validator_index": "3",
      |            "signature": "N9PlulBffWXcX/+ISzAQ23D1aGeXJ+zvYQBEPrv+xFG7Ouu78JaHCT+45Mp+QzdYYfj1+9WhPTpUhIVfk672AA=="
      |          }
      |        ]
      |      }
      |    }
      |  }
      |}
    """.stripMargin
}
