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

package fluence.node
import cats.{~>, Id}
import cats.effect.IO
import fluence.log.Log

import scala.language.higherKinds

trait GanacheSetup extends OsSetup {

  def wireupContract()(implicit log: Log[IO]): Unit = {
    val l = log.mapK[Id](Lambda[IO ~> Id](_.unsafeRunSync()))
    l.info("bootstrapping npm")
    runCmd("npm install")

    l.info("starting Ganache")
    runBackground("npm run ganache")

    l.info("deploying contract to Ganache")
    runCmd("npm run migrate")
  }

  def killGanache()(implicit log: Log[IO]): Unit = {
    log.info("killing ganache").unsafeRunSync()
    runCmd("pkill -f ganache")
  }
}
