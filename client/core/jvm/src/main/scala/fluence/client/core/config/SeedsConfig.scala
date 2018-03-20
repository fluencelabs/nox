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

package fluence.client.core.config

import cats.Traverse
import cats.effect.IO
import cats.instances.list._
import com.typesafe.config.Config
import fluence.crypto.signature.SignatureChecker
import fluence.kad.protocol.Contact

case class SeedsConfig(
  seeds: List[String]
) {

  def contacts(implicit checker: SignatureChecker): IO[List[Contact]] =
    Traverse[List].traverse(seeds)(s ⇒ Contact.readB64seed[IO](s).value.flatMap(IO.fromEither))
}

object SeedsConfig {

  /**
    * Reads seed nodes contacts from config
    */
  def read(conf: Config): IO[SeedsConfig] =
    IO {
      import net.ceedubs.ficus.Ficus._
      import net.ceedubs.ficus.readers.ArbitraryTypeReader._
      conf.as[SeedsConfig]("fluence.network")
    }
}
