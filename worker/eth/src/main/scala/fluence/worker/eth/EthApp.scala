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

package fluence.worker.eth

import cats.Applicative
import cats.data.EitherT
import org.web3j.abi.datatypes.generated._
import scodec.bits.ByteVector

import scala.language.higherKinds
import scala.util.Try

/**
 * Represents an App deployed to some cluster
 *
 * @param id Application ID as defined in Fluence contract
 * @param code Reference to the application code in a decentralized storage
 * @param cluster A cluster that hosts this App
 */
case class EthApp private[eth] (
  id: Long,
  code: StorageRef,
  cluster: Cluster
)

object EthApp {

  case class AppMalformedError(cause: Throwable)

  // TODO make private
  def apply[F[_]: Applicative](
    appId: Uint256,
    storageHash: Bytes32,
    storageType: Bytes32,
    cluster: Cluster
  ): EitherT[F, AppMalformedError, EthApp] =
    EitherT.fromEither(
      Try(
        EthApp(
          appId.getValue.longValueExact(),
          StorageRef(ByteVector(storageHash.getValue), ByteVector(storageType.getValue)),
          cluster
        )
      ).toEither.left.map(AppMalformedError)
    )

}
