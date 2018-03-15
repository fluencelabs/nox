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

package fluence.node.config

import java.net.InetAddress

import cats.effect.IO
import com.typesafe.config.Config
import net.ceedubs.ficus.readers.ValueReader

case class ContactConf(
    host: Option[InetAddress],
    grpcPort: Option[Int],
    gitHash: String,
    protocolVersion: Long
)

object ContactConf {
  def read(config: Config): IO[ContactConf] =
    IO {
      import net.ceedubs.ficus.Ficus._
      import net.ceedubs.ficus.readers.ArbitraryTypeReader._
      implicit val inetAddressRead: ValueReader[InetAddress] =
        (config: Config, path: String) ⇒ InetAddress.getByName(config.as[String](path))
      config.as[ContactConf]("fluence.network.contact")
    }
}
