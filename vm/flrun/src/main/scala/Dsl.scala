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

import cats.effect.concurrent.Ref
import cats.effect.{ExitCode, IO, IOApp}
import cats.syntax.list._
import fluence.statemachine.config.StateMachineConfig
import fluence.statemachine.error.VmModuleLocationError
import fluence.vm.WasmVm
import org.http4s.dsl.Http4sDsl
import org.http4s.implicits._
import org.http4s.server.blaze.BlazeServerBuilder
import org.http4s.server.middleware.{CORS, CORSConfig}
import org.http4s.{HttpApp, HttpRoutes, Response}
import scodec.bits.ByteVector
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LogLevel, LoggerConfig, PrintLoggerFactory}

import scala.concurrent.duration._

object Dsl {
  implicit val dsl: Http4sDsl[IO] = new Http4sDsl[IO] {}

  import dsl._

  object QueryPath extends QueryParamDecoderMatcher[String]("path")
  object QueryData extends OptionalQueryParamDecoderMatcher[String]("data")
  object QueryId extends QueryParamDecoderMatcher[String]("id")
}
