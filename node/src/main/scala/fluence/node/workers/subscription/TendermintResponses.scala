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
import cats.syntax.either._
import fluence.statemachine.api.query.QueryCode
import fluence.statemachine.api.tx.TxCode
import io.circe.generic.semiauto.deriveDecoder

import scala.util.Try

/**
 * Represents response code for broadcastTxSync.
 *
 */
case class TxResponseCode(code: Option[TxCode.Value], info: Option[String])

object TxResponseCode {
  implicit val codeDecoder: Decoder[TxCode.Value] =
    Decoder.decodeInt.emap(
      i => Try(TxCode(i)).toEither.leftMap(err => s"Cannot parse code '$i' to TxCode enum: " + err.getMessage)
    )
  implicit val decodeTxResponseCode: Decoder[TxResponseCode] = new Decoder[TxResponseCode] {
    final def apply(c: HCursor): Decoder.Result[TxResponseCode] =
      c.downField("result").as[TxResponseCode](deriveDecoder[TxResponseCode])
  }
}

/**
 * Represents response code for ABCI_query.
 *
 */
case class QueryResponseCode(code: Option[QueryCode.Value], info: Option[String])

object QueryResponseCode {
  implicit val codeDecoder: Decoder[QueryCode.Value] =
    Decoder.decodeInt.emap(
      i => Try(QueryCode(i)).toEither.leftMap(err => s"Cannot parse code '$i' to QueryCode enum: " + err.getMessage)
    )
  implicit val decodeQueryResponseCode: Decoder[QueryResponseCode] = new Decoder[QueryResponseCode] {
    final def apply(c: HCursor): Decoder.Result[QueryResponseCode] =
      c.downField("result").downField("response").as[QueryResponseCode](deriveDecoder[QueryResponseCode])
  }
}
