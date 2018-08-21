/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.statemachine.tx

import com.google.protobuf.ByteString
import fluence.statemachine.contract.ClientRegistry
import fluence.statemachine.util.{Crypto, HexCodec}
import io.circe.generic.auto._
import io.circe.parser._

import scala.util.Try

/**
 * Parser of incoming `rawTx` into verified transaction.
 * Returns error message if parsing failed (because of wrong format or incorrect signature).
 *
 * Does not perform any checks against application state.
 *
 * @param clientRegistry client registry used to check client's signature
 */
class TxParser(clientRegistry: ClientRegistry) {

  /**
   * Tries to parse a given serialized transaction.
   *
   * @param rawTx serialized transaction's raw bytes
   * @return either successfully parsed transaction or error message
   */
  def parseTx(rawTx: ByteString): Either[String, Transaction] = {
    for {
      txText <- Try(new String(HexCodec.hexToString(rawTx.toStringUtf8))).toEither.left.map(_.getMessage)
      parsedJson <- parse(txText).left.map(_.message)
      signedTx <- parsedJson.as[SignedTransaction].left.map(_.message)
      publicKey <- clientRegistry.getPublicKey(signedTx.tx.header.client)
      checkedTx <- Either.cond(
        Crypto.verify(signedTx.signature, signedTx.tx.signString, publicKey),
        signedTx.tx,
        "Invalid signature"
      )
    } yield checkedTx
  }

}
