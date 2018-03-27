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

package fluence.transport.grpc

import cats.ApplicativeError
import com.typesafe.config.Config
import fluence.transport.grpc.server.GrpcServerConf

import scala.language.higherKinds

// TODO: these headers should not belong to GrpcConf, as they are Kademlia-grpc-specific
case class GrpcConf(
  keyHeader: String,
  contactHeader: String,
  server: Option[GrpcServerConf]
)

object GrpcConf {
  val ConfigPath = "fluence.grpc"

  def read[F[_]](config: Config, path: String = ConfigPath)(implicit F: ApplicativeError[F, Throwable]): F[GrpcConf] =
    F.catchNonFatal {
      import net.ceedubs.ficus.Ficus._
      import net.ceedubs.ficus.readers.ArbitraryTypeReader._
      config.as[GrpcConf](path)
    }
}
