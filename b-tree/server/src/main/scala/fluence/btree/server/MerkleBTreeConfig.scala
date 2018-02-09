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

package fluence.btree.server

import cats.ApplicativeError
import com.typesafe.config.Config

import scala.language.higherKinds

/**
 * Configuration for [[MerkleBTree]]
 *
 * @param arity  Maximum size of node (max number of tree node keys)
 * @param alpha  Minimum capacity factor of node. Should be between 0 and 0.5.
 *               0.25 means that each node except root should always contains between 25% and 100% children.
 */
case class MerkleBTreeConfig(
    arity: Int = 8,
    alpha: Double = 0.25D
) {
  require(arity % 2 == 0, "arity should be even")
  require(0 < alpha && alpha < 0.5, "alpha should be between 0 and 0.5")
}

object MerkleBTreeConfig {

  val ConfigPath = "fluence.merkle.btree"

  def read[F[_]](conf: Config, configPath: String = ConfigPath)(implicit F: ApplicativeError[F, Throwable]): F[MerkleBTreeConfig] =
    F.catchNonFatal {
      import net.ceedubs.ficus.Ficus._
      import net.ceedubs.ficus.readers.ArbitraryTypeReader._
      conf.as[MerkleBTreeConfig](configPath)
    }

}
