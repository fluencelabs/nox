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

package fluence.effects.ipfs

import cats.MonadError
import cats.effect.IO
import cats.syntax.monadError._
import cats.syntax.try_._
import com.softwaremill.sttp.{SttpBackend, TryHttpURLConnectionBackend}
import com.softwaremill.sttp._
import fluence.effects.castore.StoreError

import scala.language.implicitConversions
import scala.util.Try

class TestMain extends App {
  implicit val sttp: SttpBackend[Try, Nothing] = TryHttpURLConnectionBackend()



  val store = new IpfsStore[Try](uri"http://data.fluence.one:5001")
}
