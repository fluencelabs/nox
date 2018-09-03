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
import java.lang.reflect.Method

import asmble.compile.jvm.AsmExtKt
import cats.data.EitherT
import cats.effect.{IO, LiftIO}
import cats.{Functor, Monad}
import fluence.crypto.Crypto.Hasher
import fluence.vm.VmError.{InternalVmError, InvalidArgError, NoSuchFnError, TrapError}
import fluence.vm.WasmVmImpl._
import scodec.bits.ByteVector

import scala.language.higherKinds
import scala.util.Try

/**
 * Base implementation of [[WasmVm]].
 *
 * '''Note!!! This implementation isn't thread-safe. The provision of calls
 * linearization is the task of the caller side.'''
 *
 * @param functionsIndex the index for fast searching function. Contains all the
 *                     information needed to execute any function.
 * @param hasher a hash function provider
 */
class WasmVmImpl(
  functionsIndex: WasmFnIndex,
  modulesIndex: WasmModuleIndex,
  hasher: Hasher[Array[Byte], Array[Byte]]
) extends WasmVm {

  override def invoke[F[_]: LiftIO: Monad](
    moduleName: Option[String],
    fnName: String,
    fnArgs: Seq[String]
  ): EitherT[F, VmError, Option[Any]] = {

    val fnId = FunctionId(moduleName, AsmExtKt.getJavaIdent(fnName))

    for {
      // Finds java method(wast fn) in the index by function id
      wasmFn <- EitherT
        .fromOption(
          functionsIndex.get(fnId),
          VmError(s"Unable to find a function with the name=$fnId", NoSuchFnError)
        )

      arguments ← parseArguments(fnArgs, wasmFn)

      // invoke the function
      result ← wasmFn[F](arguments)
    } yield if (wasmFn.javaMethod.getReturnType == Void.TYPE) None else Option(result)

  }

  /**
   * Returns hash of significant inner state of this VM. This function
   * creates a hash for each module state and then in series concatenates its together.
   * {{{
   *   vmStateHash = hash( hash( hash(module1), hash(module2) ), ...))
   * }}}
   * '''Note!''' It's very expensive operation try to avoid frequent use.
   */
  override def getVmState[F[_]: LiftIO: Monad]: EitherT[F, VmError, ByteVector] =
    modulesIndex
      .foldLeft(EitherT.rightT[F, VmError](Array[Byte]())) {
        case (acc, instance) ⇒
          for {
            moduleStateHash <- instance.innerState(arr ⇒ hasher[F](arr))

            prevModulesHash ← acc

            concatHashes ← safeConcat(moduleStateHash, prevModulesHash)

            resultHash ← hasher(concatHashes)
              .leftMap(e ⇒ VmError(s"Getting VM state for module=$instance failed", Some(e), InternalVmError))

          } yield resultHash
      }
      .map(ByteVector(_))

}

object WasmVmImpl {

  type WasmModuleIndex = List[ModuleInstance]
  type WasmFnIndex = Map[FunctionId, WasmFunction]

  /** Function id contains two components optional module name and function name. */
  case class FunctionId(moduleName: Option[String], functionName: String) {
    override def toString =
      s"'${ModuleInstance.nameAsStr(moduleName)}.$functionName'"
  }

  /**
   * Representation for each WASM function. Contains inside reference to module
   * instance and java method [[java.lang.reflect.Method]].
   *
   * @param functionId a full name of this function (moduleName + functionName)
   * @param javaMethod a java method [[java.lang.reflect.Method]] for calling fn.
   * @param module the object the underlying method is invoked from.
   *               This is an instance for the current module, it contains
   *               all inner state of the module, like memory.
   */
  case class WasmFunction(
    functionId: FunctionId,
    javaMethod: Method,
    module: ModuleInstance
  ) {

    /**
     * Invokes this function with arguments.
     *
     * @param args arguments for calling this fn.
     * @tparam F a monad with an ability to absorb 'IO'
     */
    def apply[F[_]: Functor: LiftIO](args: List[AnyRef]): EitherT[F, VmError, AnyRef] =
      EitherT(IO(javaMethod.invoke(module.instance, args: _*)).attempt.to[F])
        .leftMap(e ⇒ VmError(s"Function $this with args: $args was failed", Some(e), TrapError))

    override def toString: String = functionId.toString
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
      .map(e ⇒ VmError(errorMsg, Some(e), InvalidArgError))

  /** Safe array concatenation. Prevents  array overflow or OOM. */
  private def safeConcat[F[_]: LiftIO: Monad](
    first: Array[Byte],
    second: Array[Byte]
  ): EitherT[F, VmError, Array[Byte]] = {
    EitherT.fromEither {
      Try(Array.concat(second, first)).toEither
    }.leftMap(e ⇒ VmError("Arrays concatenation failed", Some(e), InternalVmError))
  }

  private def parseArguments[F[_]: LiftIO: Monad](
    fnArgs: Seq[String],
    wasmFn: WasmFunction
  ): EitherT[F, VmError, List[AnyRef]] = {

    // Maps args to params
    val required = wasmFn.javaMethod.getParameterTypes.length

    for {
      _ ← EitherT.cond(
        required == fnArgs.size,
        (),
        VmError(
          s"Invalid number of arguments, expected=$required, actually=${fnArgs.size} for fn=$wasmFn",
          InvalidArgError
        )
      )

      args = wasmFn.javaMethod.getParameterTypes.zipWithIndex.zip(fnArgs).map {
        case ((paramType, index), arg) =>
          paramType match {
            case cl if cl == classOf[Int] ⇒ cast(arg, _.toInt, s"Arg $index of '$arg' not an int")
            case cl if cl == classOf[Long] ⇒ cast(arg, _.toLong, s"Arg $index of '$arg' not a long")
            case cl if cl == classOf[Float] ⇒ cast(arg, _.toFloat, s"Arg $index of '$arg' not a float")
            case cl if cl == classOf[Double] ⇒ cast(arg, _.toDouble, s"Arg $index of '$arg' not a double")
            case _ ⇒
              Left(
                VmError(
                  s"Invalid type for param $index: $paramType, expected [i32|i64|f32|f64]",
                  InvalidArgError
                )
              )
          }
      }

      result ← list2Either[F, VmError, AnyRef](args.toList)

    } yield result

  }

}
