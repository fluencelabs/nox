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

package fluence.statemachine.tx

import java.nio.charset.Charset

import cats.Applicative
import cats.data.EitherT
import com.google.protobuf.ByteString
import fluence.statemachine.contract.ClientRegistry
import fluence.statemachine.util.{ClientInfoMessages, Crypto, HexCodec}
import io.circe.generic.auto._
import io.circe.parser.{parse => parseJson}

import scala.language.higherKinds

/**
 * Parser of incoming `rawTx` into verified transaction.
 * Returns error message if parsing failed (because of wrong format or incorrect signature).
 *
 * Does not perform any checks against application state.
 *
 * @param clientRegistry client registry used to check client's signature
 */
class TxParser[F[_]: Applicative](clientRegistry: ClientRegistry) extends slogging.LazyLogging {

  /**
   * Tries to parse a given serialized transaction.
   *
   * @param rawTx serialized transaction's raw bytes
   * @return either successfully parsed transaction or error message
   */
  def parseTx(rawTx: ByteString): EitherT[F, String, Transaction] =
    EitherT.fromEither[F](for {
      _ <- Right(())
      _ = logger.info("RAWTX === " + rawTx.toStringUtf8)
      _ = logger.info("AFTER CODEC === " + HexCodec.hexToString(rawTx.toStringUtf8))
      txText <- HexCodec.hexToString(rawTx.toStringUtf8)
      _ = logger.info("TX TEXT === " + txText)
      parsedJson <- parseJson(txText).left.map(_.message)
      _ = logger.info("PARSED JSON === " + parsedJson)
      signedTx <- parsedJson.as[SignedTransaction].left.map(_.message)
      publicKey <- clientRegistry.getPublicKey(signedTx.tx.header.client)
      checkedTx <- Either.cond(
        Crypto.verify(signedTx.signature, signedTx.tx.signString, publicKey),
        signedTx.tx,
        ClientInfoMessages.InvalidSignature
      )
    } yield checkedTx)

}
