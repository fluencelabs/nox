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

package fluence.kvstore.rocksdb

import cats.Monad
import cats.data.EitherT
import cats.syntax.functor._
import com.typesafe.config.Config
import fluence.kvstore.StoreError

import scala.language.higherKinds
import scala.util.Try

case class RocksDbConf(dataDir: String, createIfMissing: Boolean)

object RocksDbConf {

  import net.ceedubs.ficus.Ficus._
  import net.ceedubs.ficus.readers.ArbitraryTypeReader._

  val ConfigPath = "fluence.storage.rocksDb"

  def read[F[_]: Monad](conf: Config, name: String = ConfigPath): EitherT[F, StoreError, RocksDbConf] =
    EitherT(
      Monad[F].pure(()).map(_ ⇒ Try(conf.as[RocksDbConf](name)).toEither)
    ).leftMap(err ⇒ StoreError(s"RocksDb initialisation exception for configPath=$name", Some(err)))

}
