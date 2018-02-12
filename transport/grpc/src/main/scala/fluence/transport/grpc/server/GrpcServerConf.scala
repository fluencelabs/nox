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

package fluence.transport.grpc.server

import cats.ApplicativeError
import com.typesafe.config.Config

import scala.language.higherKinds
import scala.util.Try

case class GrpcServerConf(
    localPort: Int,
    externalPort: Option[Int],
    acceptLocal: Boolean,

    protocolVersion: Long,
    gitHash: String
) {
  require(gitHash.length == 20, "Git hash length must be 20 symbols")
}

object GrpcServerConf {
  val ConfigPath = "fluence.transport.grpc.server"
  val ConfigProtocolVersionPath = "fluence.transport.grpc.protocolVersion"
  val ConfigGitHashPath = "fluence.gitHash"

  case class GrpcServerConfSubset(
      localPort: Int,
      externalPort: Option[Int],
      acceptLocal: Boolean)

  def read[F[_]](
    config: Config,
    path: String = ConfigPath,
    protocolVersionPath: String = ConfigProtocolVersionPath,
    gitHashPath: String = ConfigGitHashPath)(implicit F: ApplicativeError[F, Throwable]): F[GrpcServerConf] =
    F.catchNonFatal {
      import net.ceedubs.ficus.Ficus._
      import net.ceedubs.ficus.readers.ArbitraryTypeReader._
      val conf = config.as[GrpcServerConfSubset](path)

      GrpcServerConf(
        conf.localPort,
        conf.externalPort,
        conf.acceptLocal,
        Try(config.getLong(protocolVersionPath)).getOrElse(0),
        Try(config.getString(gitHashPath)).getOrElse("0" * 20)
      )
    }
}
