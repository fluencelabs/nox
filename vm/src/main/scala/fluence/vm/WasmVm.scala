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
import asmble.run.jvm.Module.{Compiled, Native}
import asmble.run.jvm.ScriptContext
import asmble.util.Logger
import cats.data.{EitherT, NonEmptyList}
import cats.effect.LiftIO
import cats.{Monad, Traverse}
import cats.instances.list._
import cats.syntax.either._
import com.typesafe.config.{Config, ConfigFactory}
import fluence.crypto.Crypto
import fluence.crypto.hash.JdkCryptoHasher
import fluence.log.Log
import fluence.merkle.TrackingMemoryBuffer
import fluence.vm.VmError.{InitializationError, NoSuchModuleError}
import fluence.vm.VmError.WasmVmError.{ApplyError, GetVmStateError, InvokeError}
import fluence.vm.wasm.{module, MemoryHasher}
import fluence.vm.config.VmConfig
import fluence.vm.utils.safelyRunThrowable
import fluence.vm.wasm.module.{EnvModule, MainWasmModule, WasmModule}
import scodec.bits.ByteVector

import scala.collection.convert.ImplicitConversionsToJava.`seq AsJavaList`
import scala.collection.convert.ImplicitConversionsToScala.`list asScalaBuffer`
import scala.language.higherKinds

/**
 * Represents VM execution result.
 */
case class InvocationResult(output: Array[Byte], spentGas: Long)

/**
 * Virtual Machine api.
 */
trait WasmVm {

  /**
   * Invokes Wasm ''function'' from specified Wasm ''module''. Each function receives and returns array of bytes.
   *
   * Note that, modules should be registered when VM started!
   *
   * @param fnArgument a Function arguments
   * @tparam F a monad with an ability to absorb 'IO'
   */
  def invoke[F[_]: LiftIO: Monad](
    fnArgument: Array[Byte] = Array.emptyByteArray
  ): EitherT[F, InvokeError, InvocationResult]

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

object WasmVm {

  /**
   * Main method factory for building VM.
   * Compiles all files immediately by Asmble and returns VM implementation with eager module instantiation.
   *
   * @param inFiles input files in wasm or wast format
   * @param configNamespace a path of config in 'lightbend/config terms, please see reference.conf
   * @param memoryHasher a hash function provider for calculating memory's hash
   * @param cryptoHasher a hash function provider for module state hash calculation
   */
  def apply[F[_]: Monad: Log](
    inFiles: NonEmptyList[String],
    memoryHasher: MemoryHasher.Builder[F],
    configNamespace: String = "fluence.vm.client",
    cryptoHasher: Crypto.Hasher[Array[Byte], Array[Byte]] = JdkCryptoHasher.Sha256,
    conf: ⇒ Config = ConfigFactory.load()
  ): EitherT[F, ApplyError, WasmVm] =
    for {
      // reading config
      config ← VmConfig.readT[F](configNamespace, conf)

      _ ← Log.eitherT[F, ApplyError].info("WasmVm: configs read...")

      // Compiling Wasm modules to JVM bytecode and registering derived classes
      // in the Asmble engine. Every Wasm module is compiled to exactly one JVM class.
      scriptCxt ← safelyRunThrowable[F, ScriptContext, ApplyError](
        prepareContext(inFiles, config),
        err ⇒
          InitializationError(
            s"Preparing execution context before execution was failed for $inFiles.",
            Some(err)
          ): ApplyError
      )

      _ ← Log.eitherT[F, ApplyError].info("WasmVm: scriptCtx prepared...")

      (mainModule, envModule, sideModules) ← initializeModules(scriptCxt, config, memoryHasher)

      _ ← Log.eitherT[F, ApplyError].info("WasmVm: modules initialized")
    } yield
      new AsmbleWasmVm(
        mainModule,
        envModule,
        sideModules,
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
        config.specTestEnabled,
        config.defaultMaxMemPages,
        config.loggerModuleEnabled,
        (capacity: Int) ⇒ TrackingMemoryBuffer.allocateDirect(capacity, config.chunkSize)
      )
    )
  }

  /**
   * This method initializes every module and builds a module index. The index is actually a map where the key is a
   * string "Some(moduleName)" and value is a [[WasmFunction]] instance. Module name can be "None" if the module
   * name wasn't specified (note that it also can be empty).
   */
  private def initializeModules[F[_]: Monad](
    ctx: ScriptContext,
    config: VmConfig,
    memoryHasher: MemoryHasher.Builder[F]
  ): EitherT[F, ApplyError, (MainWasmModule, EnvModule, Seq[WasmModule])] =
    for {

      rawEnvModule ← EitherT.cond[F](
        ctx.getRegistrations.containsKey(config.envModuleConfig.name),
        ctx.getRegistrations.get(config.envModuleConfig.name),
        NoSuchModuleError(
          s"Asmble doesn't provide the environment module with name=${config.envModuleConfig.name} (perhaps you are using the old version)"
        ): ApplyError
      )

      nativeModule <- EitherT.fromEither[F](rawEnvModule match {
        case m: Native => m.asRight
        case _ =>
          NoSuchModuleError(
            s"Environment module ${config.envModuleConfig.name} was found, but it isn't a Native module"
          ).asLeft
      })

      envModule ← EnvModule[F](
        nativeModule,
        ctx,
        config.envModuleConfig.spentGasFunctionName,
        config.envModuleConfig.clearStateFunction
      )

      modulesCount = ctx.getModules.size()

      (mainModule, sideModules) ← Traverse[List]
        .foldLeftM[EitherT[F, ApplyError, ?], Compiled, (Option[MainWasmModule], List[WasmModule])](
          ctx.getModules.toList,
          (None, Nil)
        ) {
          // the main module almost always doesn't have name section (in config it is represented by None)
          // also if there is only one module provided, it is considered as the main regardless of its name.
          case ((None, sideModules), moduleDescription)
              if Option(moduleDescription.getName) == config.mainModuleConfig.name || modulesCount == 1 ⇒
            for {
              mainModule ← MainWasmModule(
                moduleDescription,
                ctx,
                memoryHasher,
                config.mainModuleConfig.allocateFunctionName,
                config.mainModuleConfig.deallocateFunctionName,
                config.mainModuleConfig.invokeFunctionName
              )
            } yield (Some(mainModule), sideModules)

          // check for the uniqueness of the main module
          case ((Some(_), _), moduleDescription) if Option(moduleDescription.getName) == config.mainModuleConfig.name ⇒
            EitherT.leftT(
              InitializationError(
                s"There should be only one main module (main module is a module without name section)"
              )
            )

          // side modules
          case ((mainModule, sideModules), moduleDescription) ⇒
            for {
              module ← module.WasmModule(
                moduleDescription,
                ctx,
                memoryHasher
              )
            } yield (mainModule, sideModules :+ module)

        }
        .flatMap[ApplyError, (MainWasmModule, Seq[WasmModule])] {
          case (Some(mainModule), sideModules) ⇒ EitherT.pure[F, ApplyError]((mainModule, sideModules))
          case _ ⇒
            EitherT.leftT(
              InitializationError(s"Please add the main module (module without name section) to the supplied modules"): ApplyError
            )
        }

    } yield (mainModule, envModule, sideModules)

}
