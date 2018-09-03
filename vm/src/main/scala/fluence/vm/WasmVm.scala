/*
 * Copyright (C) 2018  Fluence Labs Limited
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

package fluence.vm

import java.lang.reflect.Modifier

import asmble.cli.Invoke
import asmble.cli.ScriptCommand.ScriptArgs
import asmble.run.jvm.ScriptContext
import asmble.util.Logger
import cats.data.EitherT
import cats.effect.LiftIO
import cats.{Applicative, Id, Monad}
import fluence.crypto.Crypto
import fluence.crypto.hash.JdkCryptoHasher
import fluence.vm.VmError.{apply ⇒ _, _}
import fluence.vm.WasmVmImpl._
import fluence.vm.config.VmConfig
import fluence.vm.config.VmConfig._
import fluence.vm.config.VmConfig.ConfigError
import scodec.bits.ByteVector

import scala.collection.convert.ImplicitConversionsToJava.`seq AsJavaList`
import scala.collection.convert.ImplicitConversionsToScala.`list asScalaBuffer`
import scala.language.higherKinds
import scala.util.Try

/**
 * Virtual Machine api.
 */
trait WasmVm {

  /**
   * Invokes ''function'' from specified ''module'' with this arguments.
   * Returns ''None'' if function don't returns result, ''Some(Any)'' if
   * function returns result, ''VmError'' when something goes wrong.
   *
   * Note that, modules and functions should be registered when VM started!
   *
   * @param module a Module name, if absent will be used last from registered modules
   * @param function a Function name to invocation
   * @param fnArgs a Function arguments
   * @tparam F a monad with an ability to absorb 'IO'
   */
  def invoke[F[_]: LiftIO: Monad](
    module: Option[String],
    function: String,
    fnArgs: Seq[String] = Nil
  ): EitherT[F, VmError, Option[Any]]
  // todo consider specifying expected return type as method arg
  // or create an overloaded method for each possible return type

  /**
   * Returns hash of significant inner state of this VM. This function first
   * creates a hash for each module state and then concatenates its together.
   * It's behaviour will change in future, till it looks like this:
   * {{{
   *   vmState = hash(hash(module1 state), hash(module2 state), ...))
   * }}}
   * '''Note!''' It's very expensive operation try to avoid frequent use.
   */
  def getVmState[F[_]: LiftIO: Monad]: EitherT[F, VmError, ByteVector]

}

object WasmVm {

  /**
   * Contains all initialized modules and index for fast function searching.
   *
   * @param modules loaded and initialized modules
   * @param functions an index for fast searching public wasm functions
   */
  private[vm] case class VmProps(
    modules: List[ModuleInstance] = Nil,
    functions: WasmFnIndex = Map()
  )

  /**
   * Main method factory for building VM.
   * Compiles all files immediately and returns VM implementation with eager
   * module instantiation, also builds index for each wast function.
   *
   * @param inFiles input files in WASM or wast format
   * @param configNamespace a path of config in 'lightbend/config terms, see reference.conf
   * @param cryptoHasher a hash function provider
   */
  def apply[F[_]: Monad](
    inFiles: Seq[String],
    configNamespace: String = "fluence.vm.client",
    cryptoHasher: Crypto.Hasher[Array[Byte], Array[Byte]] = JdkCryptoHasher.Sha256
  ): EitherT[F, VmError, WasmVm] = {

    for {
      // reading config
      config ← EitherT
        .fromEither(pureconfig.loadConfig[VmConfig](configNamespace))
        .leftMap { e ⇒
          VmError(
            s"Unable to read a config for the namespace=$configNamespace",
            Some(ConfigError(e)),
            InternalVmError
          )
        }

      // Compiling WASM modules to JVM bytecode and registering derived classes
      // in the Asmble engine. Every WASM module compiles to exactly one JVM class
      scriptCxt ← run(
        prepareContext(inFiles, config),
        s"Preparing execution context before execution was failed for $inFiles.",
        InitializationError
      )

      // initializing all modules, build index for all wasm functions
      vmProps ← initializeModules(scriptCxt)

    } yield
      new WasmVmImpl(
        vmProps.functions,
        vmProps.modules,
        cryptoHasher
      )
  }

  /**
   * Returns [[ScriptContext]] - context for uploaded wasm modules .
   * Compiling WASM modules to JVM bytecode and registering derived classes
   * in the Asmble engine. Every WASM module compiles to exactly one JVM class
   */
  private def prepareContext(
    inFiles: Seq[String],
    config: VmConfig
  ): ScriptContext = {
    val invoke = new Invoke()
    // todo in future should be used common logger for this project
    val logger = new Logger.Print(Logger.Level.WARN)
    invoke.setLogger(logger)
    invoke.prepareContext(
      new ScriptArgs(
        inFiles,
        Nil, // registrations
        false, // disableAutoRegister
        config.specTestRegister,
        config.defaultMaxMemPages
      )
    )
  }

  /**
   * This method initializes every module and builds a total index for each
   * function of every module. The index is actually a map where the key is a
   * string "Some(moduleName), fnName)" and value is a [[WasmFunction]] instance.
   * Module name can be "None" if the module name wasn't specified, in this case,
   * there aren't to be 2 modules without names that contain functions with the
   * same names, otherwise, an error will be thrown.
   */
  private def initializeModules[F[_]: Applicative](
    scriptCxt: ScriptContext
  ): EitherT[F, VmError, VmProps] = {

    val emptyIndex: Either[VmError, VmProps] = Right(VmProps())

    val filledIndex = scriptCxt.getModules.foldLeft(emptyIndex) {

      case (error @ Left(_), _) ⇒
        // if error already occurs, skip next module and return the previous error.
        error

      case (Right(vmProps), moduleDesc) ⇒
        for {
          // initialization module instance
          moduleInstance <- ModuleInstance(moduleDesc, scriptCxt)
          // building module index for fast access to functions
          methodsAsWasmFns = moduleDesc.getCls.getDeclaredMethods
            .withFilter(m ⇒ Modifier.isPublic(m.getModifiers))
            .map { method ⇒
              val fnId = FunctionId(moduleInstance.name, method.getName)

              vmProps.functions.get(fnId) match {

                case None ⇒
                  // it's ok, function with the specified name wasn't registered yet
                  val fn = WasmFunction(
                    functionId = fnId,
                    method,
                    moduleInstance
                  )
                  Right(fnId → fn)

                case Some(fn) ⇒
                  // module and function with the specified name were already registered
                  // this situation is unacceptable, raise the error
                  Left(VmError(s"The function $fn was already registered", InitializationError))
              }

            }

          moduleFunctions ← list2Either[Id, VmError, (FunctionId, WasmFunction)](methodsAsWasmFns.toList).value

        } yield
          VmProps(
            vmProps.modules :+ moduleInstance,
            vmProps.functions ++ moduleFunctions
          )

    }

    EitherT.fromEither[F](filledIndex)
  }

  /** Helper method. Run ''action'' inside Try block, convert to EitherT with specified effect F */
  private def run[F[_]: Applicative, T](
    action: ⇒ T,
    errorMsg: String,
    errorKind: VmErrorKind
  ): EitherT[F, VmError, T] =
    EitherT
      .fromEither(Try(action).toEither)
      .leftMap(cause ⇒ VmError(errorMsg, Some(cause), errorKind))

}
