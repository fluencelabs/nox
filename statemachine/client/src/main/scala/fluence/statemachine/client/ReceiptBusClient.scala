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

package fluence.statemachine.client

import cats.Monad
import cats.data.EitherT
import cats.syntax.functor._
import fluence.effects.EffectError
import fluence.effects.sttp.SttpEffect
import fluence.effects.sttp.syntax._
import com.softwaremill.sttp.{sttp, _}
import fluence.log.Log
import fluence.statemachine.api.command.ReceiptBus
import fluence.statemachine.api.data.BlockReceipt
import scodec.bits.ByteVector
import com.softwaremill.sttp.circe._

import scala.language.higherKinds

class ReceiptBusClient[F[_]: Monad: SttpEffect](host: String, port: Short) extends ReceiptBus[F] {

  override def getVmHash(height: Long)(implicit log: Log[F]): EitherT[F, EffectError, ByteVector] =
    sttp
      .get(uri"http://$host:$port/receipt-bus/getVmHash?height=$height")
      .send()
      .decodeBody(ByteVector.fromHex(_).toRight(new RuntimeException("Not a hex")))
      .leftMap(identity[EffectError])

  override def sendBlockReceipt(receipt: BlockReceipt)(implicit log: Log[F]): EitherT[F, EffectError, Unit] =
    sttp
      .post(uri"http://$host:$port/receipt-bus/blockReceipt")
      .body(receipt)
      .send()
      .toBody
      .void
      .leftMap(identity[EffectError])
}
