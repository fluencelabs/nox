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

import cats.effect._
import fluence.worker.WorkerStatus

import scala.io.Source
import scala.language.higherKinds

trait GetWorkerStatus {
  protected def getWorkerStatus(host: String, port: Short, appId: Long): IO[WorkerStatus] = IO {
    import io.circe.parser.parse
    val url = s"http://$host:$port/apps/$appId/status"
    val source = {
      val s = Source.fromURL(url)
      val src = s.mkString
      s.close()
      src
    }

    parse(source).right.get.as[WorkerStatus].right.get
  }
}
