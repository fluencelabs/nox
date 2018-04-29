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

package fluence.client.cli

import java.io.File

import fluence.crypto.keystore.KeyStore
import scopt.Read.reads
import scopt.{OptionParser, Read}

// TODO: actually it isn't used
object ArgsParser {
  case class CommandLineConfig(
    config: Option[File] = None,
    seed: Seq[String] = Seq.empty,
    keyStore: Option[KeyStore] = None
  )

  implicit val keyStoreRead: Read[KeyStore] = {
    reads { str ⇒
      KeyStore.fromBase64(str).value.toTry.get // TODO: this is impure
    }
  }

  private lazy val parser = new OptionParser[CommandLineConfig]("fluenceClient") {
    head("Fluence client")

    opt[File]('c', "config")
      .valueName("<file>")
      .action((x, c) ⇒ c.copy(config = Some(x)))
      .text("Path to config file")

    opt[Seq[String]]('s', "seed")
      .valueName("<seed1>,<seed2>...")
      .action((x, c) ⇒ c.copy(seed = x))
      .text("Initial kademlia nodes contacts in base64 to connect with")

    opt[KeyStore]('k', "keystore")
      .valueName("<keystore>")
      .action((x, c) ⇒ c.copy(keyStore = Some(x)))
      .text("Key pair in base64")

    help("help").text("Write help message")

    note("Arguments will override values in config file")
  }

  def parse(args: Array[String]) = parser.parse(args, CommandLineConfig())
}
