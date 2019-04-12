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

import org.http4s.server.middleware.CORSConfig

import scala.concurrent.duration._

object Settings {

  val CodeDirectory =
    "/Users/folex/Development/fluencelabs/fluence-main/vm/src/it/resources/test-cases/llamadb/target/wasm32-unknown-unknown/release/"
  //  val CodeDirectory = "Code"
  val Port = 30000
  val Host = "0.0.0.0"

  val corsConfig = CORSConfig(
    anyOrigin = true,
    anyMethod = true,
    allowedMethods = Some(Set("GET", "POST")),
    allowCredentials = true,
    maxAge = 1.day.toSeconds
  )
}
