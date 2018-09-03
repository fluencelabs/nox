/*
 * Copyright (C) 2017  Fluence Labs Limited
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

import java.lang.reflect.{Method, Modifier}

import asmble.cli.Invoke
import asmble.cli.ScriptCommand.ScriptArgs
import asmble.compile.jvm.AsmExtKt
import asmble.run.jvm.ScriptContext
import cats.data.EitherT
import cats.effect.{IO, LiftIO}
import cats.{Applicative, Functor, Id, Monad}
import fluence.vm.VmError.{apply ⇒ _, _}
import fluence.vm.config.VmConfig
import fluence.vm.config.VmConfig._
import fluence.vm.config.VmConfig.ConfigError

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
   * @param module A Module name, if absent will be used last from registered modules
   * @param function A Function name to invocation
   * @param fnArgs A Function arguments
   * @tparam F A monad with an ability to absorb 'IO'
   */
  def invoke[F[_]: LiftIO: Monad](
    module: Option[String],
    function: String,
    fnArgs: Seq[String]
  ): EitherT[F, VmError, Option[Any]]
  // todo consider specifying expected return type as method arg
  // or create an overloaded method for each possible return type

}

object WasmVm {

  private type WasmFnIndex = Map[FunctionId, WasmFunction]

  /** Function id contains two components optional module name and function name. */
  private case class FunctionId(moduleName: Option[String], functionName: String) {

    override def toString: String =
      s"'${moduleName.getOrElse("<unknown>")}.$functionName'"
  }

  /**
   * Representation for each WASM function. Contains inside reference to module
   * instance and java method [[java.lang.reflect.Method]].
   *
   * @param functionId A full name of this function (moduleName + functionName)
   * @param javaMethod A java method [[java.lang.reflect.Method]] for calling fn.
   * @param moduleInstance The object the underlying method is invoked from.
   *                       This is an instance for the current module, it contains
   *                       all inner state of the module, like memory.
   */
  private case class WasmFunction(
    functionId: FunctionId,
    javaMethod: Method,
    private val moduleInstance: AnyRef
  ) {

    /**
     * Invokes this function with arguments.
     *
     * @param args Arguments for calling this fn.
     * @tparam F A monad with an ability to absorb 'IO'
     */
    def apply[F[_]: Functor: LiftIO](args: List[AnyRef]): EitherT[F, VmError, AnyRef] =
      EitherT(IO(javaMethod.invoke(moduleInstance, args: _*)).attempt.to[F])
        .leftMap(e ⇒ VmError(s"Function $this with params: $args was failed", Some(e), Runtime))

    override def toString: String = functionId.toString
  }

  /**
   * Main method factory for building VM.
   * Compiles all files immediately and returns VM implementation with eager
   * module instantiation, also builds index for each wast function.
   *
   * @param inFiles Input files in WASM or wast format
   * @param configNamespace A path of config in 'lightbend/config terms, see reference.conf
   */
  def apply[F[_]: Monad](
    inFiles: Seq[String],
    configNamespace: String = "fluence.vm.client"
  ): EitherT[F, VmError, WasmVm] = {

    for {
      // reading config
      config ← EitherT
        .fromEither(pureconfig.loadConfig[VmConfig](configNamespace))
        .leftMap { e ⇒
          VmError(
            s"Unable to read a config for the namespace=$configNamespace",
            Some(ConfigError(e)),
            Initialization
          )
        }

      // Compiling WASM modules to JVM bytecode and registering derived classes
      // in the Asmble engine. Every WASM module compiles to exactly one JVM class
      scriptCxt ← run(
        new Invoke().prepareContext(
          new ScriptArgs(
            inFiles,
            Nil, // registrations
            false, // disableAutoRegister
            config.specTestRegister,
            config.defaultMaxMemPages
          )
        ),
        s"Preparing execution context before execution was failed for $inFiles.",
        Initialization
      )

      // initializing all modules, build index for all wast functions
      allFnsIndex ← initializeModules(scriptCxt)

    } yield
      new WasmVm() {

        /**
         * The index for fast searching function. Contains all the information
         * needed to execute any function.
         */
        private val functionsIdx: WasmFnIndex = allFnsIndex

        override def invoke[M[_]: LiftIO: Monad](
          moduleName: Option[String],
          fnName: String,
          fnArgs: Seq[String]
        ): EitherT[M, VmError, Option[Any]] = {

          val fnId = FunctionId(moduleName, AsmExtKt.getJavaIdent(fnName))

          for {
            // Finds java method(wast fn) in the index by function id
            wasmFn <- EitherT.fromOption[M](
              functionsIdx.get(fnId),
              VmError.validation(s"Unable to find a function with the name=$fnId")
            )

            // Maps args to params
            required = wasmFn.javaMethod.getParameterTypes.length
            _ ← EitherT.cond[M](
              required == fnArgs.size,
              (),
              VmError.validation(
                s"Invalid number of arguments, expected=$required, actually=${fnArgs.size} for fn=$wasmFn"
              )
            )

            // parse arguments
            params = {
              wasmFn.javaMethod.getParameterTypes.zipWithIndex.zip(fnArgs).map {
                case ((paramType, index), arg) =>
                  paramType match {
                    case cl if cl == classOf[Int] ⇒ cast[M](arg, _.toInt, s"Arg $index of '$arg' not an int")
                    case cl if cl == classOf[Long] ⇒ cast[M](arg, _.toLong, s"Arg $index of '$arg' not a long")
                    case cl if cl == classOf[Float] ⇒ cast[M](arg, _.toFloat, s"Arg $index of '$arg' not a float")
                    case cl if cl == classOf[Double] ⇒ cast[M](arg, _.toDouble, s"Arg $index of '$arg' not a double")
                    case _ =>
                      Left(
                        VmError
                          .validation(s"Invalid type for param $index: $paramType, expected [i32|i64|f32|f64]")
                      )
                  }
              }
            }
            params ← list2Either[M, VmError, AnyRef](params.toList)

            // invoke the function
            result ← wasmFn[M](params)
          } yield if (wasmFn.javaMethod.getReturnType == Void.TYPE) None else Option(result)

        }

      }

  }

  /**
   * This method initializes every module and builds a total index for each
   * function of every module. The index is actually a map where the key is a
   * string "Some(moduleName), fnName)" and value is a [[WasmFunction]] instance.
   * Module name can be "None" if the module name wasn't specified, in this case,
   * there aren't to be 2 modules without names that contain functions with the
   * same names, otherwise, an error will be thrown.
   */
  private def initializeModules[F[_]: Applicative](scriptCxt: ScriptContext): EitherT[F, VmError, WasmFnIndex] = {

    val emptyIndex: Either[VmError, WasmFnIndex] = Right(Map())

    val filledIndex = scriptCxt.getModules.foldLeft(emptyIndex) {

      case (error @ Left(_), _) ⇒
        // if error already occurs, skip next module and return the previous error.
        error

      case (Right(totalIdx), moduleDesc) ⇒
        val moduleName = Option(moduleDesc.getName)
        for {
          // initialization module instance
          moduleInstance <- Try(moduleDesc.instance(scriptCxt)).toEither.left
          // todo 'instance' must throw both an initialization error and runtime errors, but now they can't be separated
            .map(e ⇒ VmError(s"Unable to initialize module with name=$moduleName", Some(e), Initialization))

          // building module index for fast access to functions
          methodsAsWasmFns = moduleDesc.getCls.getDeclaredMethods
            .withFilter(m ⇒ Modifier.isPublic(m.getModifiers))
            .map { method ⇒
              val fnId = FunctionId(moduleName, method.getName)

              totalIdx.get(fnId) match {

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
                  Left(VmError(s"The function $fn was already registered", None, Initialization))
              }

            }

          moduleIdx ← list2Either[Id, VmError, (FunctionId, WasmFunction)](methodsAsWasmFns.toList).value

        } yield totalIdx ++ moduleIdx // adds module index to total index

    }

    EitherT.fromEither[F](filledIndex)
  }

  /** Transforms specified ''arg'' with ''castFn'', returns ''errorMsg'' otherwise. */
  private def cast[F[_]: Monad](
    arg: String,
    castFn: String ⇒ Any,
    errorMsg: String
  ): Either[VmError, AnyRef] =
    // it's important to cast to AnyRef type, because args should be used in
    // Method.invoke(...) as Object type
    Try(castFn(arg).asInstanceOf[AnyRef]).toEither.left
      .map(e ⇒ VmError.validation(errorMsg, Some(e)))

  /** Helper method. Converts List of Ether to Either of List. */
  private def list2Either[F[_]: Applicative, A, B](list: List[Either[A, B]]): EitherT[F, A, List[B]] = {
    import cats.instances.list._
    import cats.syntax.traverse._
    import cats.instances.either._
    // unfortunately Idea don't understand this and show error in Editor
    val either: Either[A, List[B]] = list.sequence
    EitherT.fromEither[F](either)
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
