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

package fluence.vm.config

import cats.data.EitherT
import cats.{Monad, MonadError}
import cats.effect.Sync
import cats.syntax.either._
import com.typesafe.config.Config
import cats.syntax.flatMap._
import cats.syntax.applicativeError._
import fluence.vm.error.InitializationError

import scala.language.higherKinds

/**
 * Main module settings.
 *
 * @param name a name of the main module (None means absence of name section in a Wasm module)
 * @param allocateFunctionName name of a function that should be called for allocation memory
 * (used for passing complex data structures)
 * @param deallocateFunctionName name of a function that should be called for deallocation
 * of previously allocated memory
 * @param invokeFunctionName name of main module handler function
 */
case class MainModuleConfig(
  name: Option[String],
  allocateFunctionName: String,
  deallocateFunctionName: String,
  invokeFunctionName: String
)

/**
 * WasmVm settings.
 *
 * @param memPagesCount the maximum count of memory pages when a module doesn't say
 * @param loggerEnabled if set, registers the logger Wasm module as 'logger'
 * @param chunkSize a size of the memory chunks, that memory will be split into
 * @param mainModuleConfig settings for the main module
 */
case class VmConfig(
  memPagesCount: Int,
  loggerEnabled: Boolean,
  chunkSize: Int,
  mainModuleConfig: MainModuleConfig
)

object VmConfig {
  import net.ceedubs.ficus.Ficus._
  import net.ceedubs.ficus.readers.ArbitraryTypeReader._

  def readT[F[_]: Monad](namespace: String, conf: ⇒ Config): EitherT[F, InitializationError, VmConfig] = {
    EitherT
      .fromEither[F](Either.catchNonFatal(conf.getConfig(namespace).as[VmConfig]))
      .leftMap(e ⇒ InitializationError("Unable to parse the virtual machine config" + e))
  }
}
