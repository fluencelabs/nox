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

package fluence.node.workers.subscription

import io.circe.{Decoder, HCursor}
import io.circe.generic.semiauto.deriveDecoder

/**
 * Represents response code for broadcastTxSync.
 *
 */
case class TxResponseCode(code: Int, info: Option[String])

object TxResponseCode {

  implicit val decodeTxResponseCode: Decoder[TxResponseCode] = new Decoder[TxResponseCode] {
    final def apply(c: HCursor): Decoder.Result[TxResponseCode] =
      c.downField("result").as[TxResponseCode](deriveDecoder[TxResponseCode])
  }
}

/**
 * Represents response code for ABCI_query.
 *
 */
case class QueryResponseCode(code: Int, info: Option[String])

object QueryResponseCode {
  implicit val decodeQueryResponseCode: Decoder[QueryResponseCode] = new Decoder[QueryResponseCode] {
    final def apply(c: HCursor): Decoder.Result[QueryResponseCode] =
      c.downField("result").downField("response").as[QueryResponseCode](deriveDecoder[QueryResponseCode])
  }
}
