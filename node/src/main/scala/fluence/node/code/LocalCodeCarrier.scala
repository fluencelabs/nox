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

package fluence.node.code

import java.nio.file.{Path, Paths}

import cats.effect.Sync
import cats.syntax.functor._
import fluence.log.Log
import fluence.worker.eth.StorageRef

import scala.language.higherKinds

/**
 * Code carrier, used for tests
 */
class LocalCodeCarrier[F[_]](implicit F: Sync[F]) extends CodeCarrier[F] {

  override def carryCode(
    ref: StorageRef,
    notUsed: Path
  )(implicit log: Log[F]): F[Path] =
    F.fromEither(ref.storageHash.decodeUtf8.map(_.trim))
      .map(p => Paths.get("/master/vmcode/vmcode-" + p)) // preloaded code in master's docker container

}
