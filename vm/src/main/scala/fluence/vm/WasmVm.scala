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

package fluence.vm

import asmble.cli.Invoke
import asmble.cli.ScriptCommand.ScriptArgs
import asmble.run.jvm.ScriptContext
import asmble.util.Logger
import cats.data.{EitherT, NonEmptyList}
import cats.effect.LiftIO
import cats.{Applicative, Monad}
import fluence.crypto.Crypto
import fluence.crypto.hash.JdkCryptoHasher
import fluence.vm.VmError.{InitializationError, InternalVmError}
import fluence.vm.VmError.WasmVmError.{ApplyError, GetVmStateError, InvokeError}
import fluence.vm.wasm.WasmFunction
import fluence.vm.config.VmConfig
import fluence.vm.config.VmConfig._
import fluence.vm.config.VmConfig.ConfigError
import fluence.vm.utils.safelyRunThrowable
import fluence.vm.wasm.WasmModule
import scodec.bits.ByteVector
import pureconfig.generic.auto._
import slogging.LazyLogging

import scala.collection.convert.ImplicitConversionsToJava.`seq AsJavaList`
import scala.collection.convert.ImplicitConversionsToScala.`list asScalaBuffer`
import scala.language.higherKinds

/**
 * Virtual Machine api.
 */
trait WasmVm {

  /**
   * Invokes Wasm ''function'' from specified Wasm ''module''. Each function receives and returns array of bytes.
   *
   * Note that, modules should be registered when VM started!
   *
   * @param module a name of Wasm module from where handle
   * @param fnArgument a Function arguments
   * @tparam F a monad with an ability to absorb 'IO'
   */
  def invoke[F[_]: LiftIO: Monad](
    module: Option[String] = None,
    fnArgument: Array[Byte] = Array.emptyByteArray
  ): EitherT[F, InvokeError, Array[Byte]]

  /**
   * Returns hash of all significant inner state of this VM. This function calculates
   * hashes for the state of each module and then concatenates them together.
   * It's behaviour will change in future, till it looks like this:
   * {{{
   *   vmState = hash(hash(module1 state), hash(module2 state), ...))
   * }}}
   * '''Note!''' It's very expensive operation, try to avoid frequent use.
   */
  def getVmState[F[_]: LiftIO: Monad]: EitherT[F, GetVmStateError, ByteVector]

}

object WasmVm extends LazyLogging {

  type ModuleIndex = Map[Option[String], WasmModule]

  /**
   * Main method factory for building VM.
   * Compiles all files immediately by Asmble and returns VM implementation with eager module instantiation.
   *
   * @param inFiles input files in wasm or wast format
   * @param configNamespace a path of config in 'lightbend/config terms, please see reference.conf
   * @param cryptoHasher a hash function provider
   */
  def apply[F[_]: Monad](
    inFiles: NonEmptyList[String],
    configNamespace: String = "fluence.vm.client",
    cryptoHasher: Crypto.Hasher[Array[Byte], Array[Byte]] = JdkCryptoHasher.Sha256
  ): EitherT[F, ApplyError, WasmVm] =
    for {
      // reading config
      config ← EitherT
        .fromEither(pureconfig.loadConfig[VmConfig](configNamespace))
        .leftMap { e ⇒
          InternalVmError(
            s"Unable to read a config for the namespace=$configNamespace",
            Some(ConfigError(e))
          )
        }

      _ = logger.info("WasmVm: configs read...")

      // Compiling Wasm modules to JVM bytecode and registering derived classes
      // in the Asmble engine. Every Wasm module is compiled to exactly one JVM class.
      scriptCxt ← safelyRunThrowable(
        prepareContext(inFiles, config),
        err ⇒
          InitializationError(
            s"Preparing execution context before execution was failed for $inFiles.",
            Some(err)
        )
      )

      _ = logger.info("WasmVm: scriptCtx prepared...")

      modules ← initializeModules(scriptCxt, config)

      _ = logger.info("WasmVm: modules initialized")
    } yield
      new AsmbleWasmVm(
        modules,
        cryptoHasher
      )

  /**
   * Returns [[ScriptContext]] - context for uploaded Wasm modules.
   * Compiles Wasm modules to JVM bytecode and registering derived classes
   * in the Asmble engine. Every Wasm module is compiled to exactly one JVM class.
   */
  private def prepareContext(
    inFiles: NonEmptyList[String],
    config: VmConfig
  ): ScriptContext = {
    val invoke = new Invoke()
    // TODO: in future common logger for this project should be used
    val logger = new Logger.Print(Logger.Level.WARN)
    invoke.setLogger(logger)
    invoke.prepareContext(
      new ScriptArgs(
        inFiles.toList,
        Nil, // registrations
        false, // disableAutoRegister
        config.specTestRegister,
        config.defaultMaxMemPages,
        config.loggerRegister
      )
    )
  }

  /**
   * This method initializes every module and builds a module index. The index is actually a map where the key is a
   * string "Some(moduleName)" and value is a [[WasmFunction]] instance. Module name can be "None" if the module
   * name wasn't specified (note that it also can be empty).
   */
  private def initializeModules[F[_]: Applicative](
    scriptCxt: ScriptContext,
    config: VmConfig
  ): EitherT[F, ApplyError, ModuleIndex] = {
    val emptyIndex: Either[ApplyError, ModuleIndex] = Right(Map[Option[String], WasmModule]())

    val moduleIndex = scriptCxt.getModules
      .foldLeft(emptyIndex) {
        case (error @ Left(_), _) ⇒
          error

        case (Right(acc), moduleDescription) ⇒
          for {
            wasmModule ← WasmModule(
              moduleDescription,
              scriptCxt,
              config.allocateFunctionName,
              config.deallocateFunctionName,
              config.invokeFunctionName
            )

          } yield acc + (wasmModule.getName → wasmModule)
      }

    EitherT.fromEither[F](moduleIndex)
  }

}
