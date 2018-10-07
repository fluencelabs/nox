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
import java.lang.reflect.Method
import java.nio.charset.Charset

import asmble.compile.jvm.AsmExtKt
import cats.data.EitherT
import cats.effect.{IO, LiftIO}
import cats.{Functor, Monad, Traverse}
import fluence.crypto.Crypto.Hasher
import fluence.vm.VmError.WasmVmError.{GetVmStateError, InvokeError}
import fluence.vm.VmError.{InvalidArgError, _}
import fluence.vm.AsmleWasmVm._
import scodec.bits.ByteVector

import scala.collection.immutable
import scala.language.higherKinds
import scala.util.Try

/**
 * Base implementation of [[WasmVm]].
 *
 * '''Note!!! This implementation isn't thread-safe. The provision of calls
 * linearization is the task of the caller side.'''
 *
 * @param functionsIndex the index for fast function searching. Contains all the
 *                     information needed to execute any function.
 * @param modules list of Wasm modules
 * @param hasher a hash function provider
 * @param allocateFunctionName name of function that will be used for allocation
 *                             memory in the Wasm part
 * @param deallocateFunctionName name of a function that will be used for freeing memory
 *                               that was previously allocated by allocateFunction
 */
class AsmleWasmVm(
  functionsIndex: WasmFnIndex,
  modules: WasmModules,
  hasher: Hasher[Array[Byte], Array[Byte]],
  allocateFunctionName: String,
  deallocateFunctionName: String
) extends WasmVm {

  // TODO: now it is assumed that allocation/deallocation functions placed together in the first module.
  // In the future it has to be refactored.
  // TODO: add handling of empty modules list.
  private val allocateFunction: Option[WasmFunction] =
    functionsIndex.get(FunctionId(modules.head.name, AsmExtKt.getJavaIdent(allocateFunctionName)))

  private val deallocateFunction: Option[WasmFunction] =
    functionsIndex.get(FunctionId(modules.head.name, AsmExtKt.getJavaIdent(deallocateFunctionName)))

  // TODO: it is need to decide how to properly get charset (maybe it is good to add "getCharset" method
  // in all wasm Modules or simply add the requirement of using fixed charset everywhere)
  private val wasmStringCharset = Charset.forName("UTF-8")

  override def invoke[F[_]: LiftIO: Monad](
    moduleName: Option[String],
    fnName: String,
    fnArgs: Seq[String]
  ): EitherT[F, InvokeError, Option[Array[Byte]]] = {
    val functionId = FunctionId(moduleName, AsmExtKt.getJavaIdent(fnName))

    for {
      // Finds java method(Wasm function) in the index by function id
      wasmFn <- EitherT
        .fromOption(
          functionsIndex.get(functionId),
          NoSuchFnError(s"Unable to find a function with the name=$functionId")
        )

      preprocessedArguments <- preprocessStringArguments(fnArgs, wasmFn.module, wasmStringCharset)
      arguments ← parseArguments(preprocessedArguments, wasmFn)

      // invoke the function
      invocationResult ← wasmFn[F](arguments)
      // TODO : In the current version it is expected that callee (Wasm module) clean memory by itself
      // after construction of the result string. F.e. in Rust String object along with &str are the common
      // classes for operations on strings, in C++ the same role has std::string and std::stringview classes.
      // So, now it is expected that Rust/C++ methods at first construct objects of corresponding classes by
      // using injected string bytes as source and then delete these string bytes. But in the future it has
      // to be done by caller through deallocate function call.

      extractedResult <- if (wasmFn.javaMethod.getReturnType == Void.TYPE) {
        EitherT.rightT[F, InvokeError](None)
      } else {
        for {
          offset <- EitherT.fromEither(Try(invocationResult.toString.toInt).toEither).leftMap { e ⇒
            NoSuchFnError(s"The Wasm allocation function returned incorrect offset=", Some(e))
          }
          extractedResult <- extractResultFromWasmModule(offset, wasmFn.module).map(Option(_))
        } yield extractedResult
      }
    } yield extractedResult

  }

  override def getVmState[F[_]: LiftIO: Monad]: EitherT[F, GetVmStateError, ByteVector] =
    modules
      .foldLeft(EitherT.rightT[F, GetVmStateError](Array[Byte]())) {
        case (acc, instance) ⇒
          for {
            moduleStateHash ← instance
              .innerState(arr ⇒ hasher[F](arr))

            prevModulesHash ← acc

            concatHashes ← safeConcat(moduleStateHash, prevModulesHash)

            resultHash ← hasher(concatHashes)
              .leftMap(e ⇒ InternalVmError(s"Getting VM state for module=$instance failed", Some(e)): GetVmStateError)

          } yield resultHash
      }
      .map(ByteVector(_))

  // TODO : In the future, it should be rewritten with cats.effect.resource
  /**
   * Allocates memory in Wasm module of supplied size by allocateFunction.
   *
   * @param size size of memory that need to be allocated
   * @tparam F a monad with an ability to absorb 'IO'
   */
  private def allocate[F[_]: LiftIO: Monad](size: Int): EitherT[F, InvokeError, AnyRef] = {
    allocateFunction match {
      case Some(fn) => fn(size.asInstanceOf[AnyRef] :: Nil)
      case _ =>
        EitherT.leftT(
          NoSuchFnError(s"Unable to find the function for memory allocation with the name=$allocateFunctionName")
        )
    }
  }

  /**
   * Deallocates previously allocated memory in Wasm module by deallocateFunction.
   *
   * @param offset address of memory to deallocate
   * @tparam F a monad with an ability to absorb 'IO'
   */
  private def deallocate[F[_]: LiftIO: Monad](offset: Int): EitherT[F, InvokeError, AnyRef] = {
    deallocateFunction match {
      case Some(fn) => fn(offset.asInstanceOf[AnyRef] :: Nil)
      case _ =>
        EitherT.leftT(
          NoSuchFnError(s"Unable to find the function for memory deallocation with the name=$deallocateFunctionName")
        )
    }
  }

  /**
   * Preprocesses each string parameter: injects it into Wasm module memory (through
   * injectStringIntoWasmModule) and replace with pointer to it in WASM module and size.
   *
   * @param functionArguments arguments for calling this fn
   * @param moduleInstance module instance used for injecting string arguments to the Wasm memory
   * @param charset charset that is used for convert the string to byte array
   * @tparam F a monad with an ability to absorb 'IO'
   */
  private def preprocessStringArguments[F[_]: LiftIO: Monad](
    functionArguments: Seq[String],
    moduleInstance: ModuleInstance,
    charset: Charset
  ): EitherT[F, InvokeError, List[String]] = {
    val result: Seq[EitherT[F, InvokeError, List[String]]] =
      functionArguments.map {
        case stringArgument
            if stringArgument.startsWith("\"")
              && stringArgument.endsWith("\"")
              && stringArgument.length >= 2 ⇒
          // it is a valid String parameter
          for {
            address <- injectStringIntoWasmModule(
              stringArgument.substring(1, stringArgument.length - 1),
              moduleInstance,
              charset
            )
          } yield List(address.toString, (stringArgument.length - 2).toString)
        case arg ⇒ EitherT.rightT[F, InvokeError](List(arg))
      }

    import cats.instances.list._
    Traverse[List].flatSequence(result.toList)
  }

  /**
   * Injects given string into Wasm module memory.
   *
   * @param injectedString string that should be inserted into Wasm module memory
   * @param moduleInstance module instance used as a provider for Wasm module memory access
   * @param charset charset that is used for convert the string to byte array
   * @tparam F a monad with an ability to absorb 'IO'
   */
  private def injectStringIntoWasmModule[F[_]: LiftIO: Monad](
    injectedString: String,
    moduleInstance: ModuleInstance,
    charset: Charset
  ): EitherT[F, InvokeError, Int] =
    for {
      // In the current version, it is possible for Wasm module to have allocation/deallocation
      // functions but doesn't have memory (ByteBuffer instance). Also since it is used getMemory
      // Wasm function for getting ByteBuffer instance, it is also possible to don't have memory
      // due to possible absence of this function in the Wasm module.
      wasmMemory <- EitherT.fromOption(
        moduleInstance.memory,
        VmMemoryError(s"Trying to use absent Wasm memory while injecting string=$injectedString")
      )

      offset <- allocate(injectedString.length)

      resultOffset <- EitherT
        .fromEither(Try {
          val convertedOffset = offset.toString.toInt
          val wasmMemoryView = wasmMemory.duplicate()

          wasmMemoryView.position(convertedOffset)
          wasmMemoryView.put(injectedString.getBytes(charset.name()))

          convertedOffset
        }.toEither)
        .leftMap { e ⇒
          VmMemoryError(s"The Wasm allocation function returned incorrect offset=$offset", Some(e))
        }: EitherT[F, InvokeError, Int]

    } yield resultOffset

  /**
   * Extracts (reads and deletes) result from the given offset from Wasm module memory.
   *
   * @param offset offset into Wasm module memory where a string is located
   * @param moduleInstance module instance used as a provider for Wasm module memory access
   * @tparam F a monad with an ability to absorb 'IO'
   */
  private def extractResultFromWasmModule[F[_]: LiftIO: Monad](
    offset: Int,
    moduleInstance: ModuleInstance
  ): EitherT[F, InvokeError, Array[Byte]] =
    for {
      extractedString <- readResultFromWasmModule(offset, moduleInstance)
      // TODO : string deallocation from scala-part should be additionally investigated - it seems
      // that this version of deletion doesn't compatible with current idea of verification game
      _ <- deallocate(offset)
    } yield extractedString

  /**
   * Reads result from the given offset from Wasm module memory.
   *
   * @param offset offset into Wasm module memory where a string is located
   * @param moduleInstance module instance used as a provider for Wasm module memory access
   * @tparam F a monad with an ability to absorb 'IO'
   */
  private def readResultFromWasmModule[F[_]: LiftIO: Monad](
    offset: Int,
    moduleInstance: ModuleInstance
  ): EitherT[F, InvokeError, Array[Byte]] =
    for {
      wasmMemory <- EitherT.fromOption(
        moduleInstance.memory,
        VmMemoryError(s"Trying to use absent Wasm memory while reading string from the offset=$offset")
      )

      readedString <- EitherT
        .fromEither[F](
          Try {
            // each string has the next structure in Wasm memory: | size (4 bytes) | string buffer (size bytes) |
            val stringSize = wasmMemory.getInt(offset)
            // size of Int in bytes
            val intBytesSize = 4

            val buffer = new Array[Byte](stringSize)
            val wasmMemoryView = wasmMemory.duplicate()
            wasmMemoryView.position(offset + intBytesSize)
            wasmMemoryView.get(buffer)
            buffer
          }.toEither
        )
        .leftMap { e =>
          VmMemoryError(
            s"String reading from offset=$offset failed",
            Some(e)
          )
        }: EitherT[F, InvokeError, Array[Byte]]

    } yield readedString

}

object AsmleWasmVm {

  type WasmModules = List[ModuleInstance]
  type WasmFnIndex = Map[FunctionId, WasmFunction]

  /** Function id contains two components: optional module name and function name. */
  case class FunctionId(moduleName: Option[String], functionName: String) {
    override def toString =
      s"'${ModuleInstance.nameAsStr(moduleName)}.$functionName'"
  }

  /**
   * Representation for each Wasm function. Contains reference to module instance
   * and java method [[java.lang.reflect.Method]].
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
    def apply[F[_]: Functor: LiftIO](args: List[AnyRef]): EitherT[F, InvokeError, AnyRef] =
      EitherT(IO(javaMethod.invoke(module.instance, args: _*)).attempt.to[F])
        .leftMap(e ⇒ TrapError(s"Function $this with args: $args was failed", Some(e)))

    override def toString: String = functionId.toString
  }

  /** Transforms specified ''arg'' with ''castFn'', returns ''errorMsg'' otherwise. */
  private def cast[F[_]: Monad](
    arg: String,
    castFn: String ⇒ Any,
    errorMsg: String
  ): Either[InvalidArgError, AnyRef] =
    // it's important to cast to AnyRef type, because args should be used in
    // Method.invoke(...) as Object type
    Try(castFn(arg).asInstanceOf[AnyRef]).toEither.left
      .map(e ⇒ InvalidArgError(errorMsg, Some(e)))

  /** Safe array concatenation. Prevents array overflow or OOM. */
  private def safeConcat[F[_]: LiftIO: Monad](
    first: Array[Byte],
    second: Array[Byte]
  ): EitherT[F, InternalVmError, Array[Byte]] = {
    EitherT.fromEither {
      Try(Array.concat(second, first)).toEither
    }.leftMap(e ⇒ InternalVmError("Arrays concatenation failed", Some(e)))
  }

  private def parseArguments[F[_]: LiftIO: Monad](
    fnArgs: List[String],
    wasmFn: WasmFunction
  ): EitherT[F, InvalidArgError, List[AnyRef]] = {

    // Maps args to params
    val required = wasmFn.javaMethod.getParameterTypes.length

    for {
      _ ← EitherT.cond(
        required == fnArgs.size,
        (),
        InvalidArgError(
          s"Invalid number of arguments, expected=$required, actually=${fnArgs.size} for fn=$wasmFn " +
            s"(or passed string argument is incorrect or isn't matched the corresponding argument in Wasm function)"
        )
      )

      args = wasmFn.javaMethod.getParameterTypes.zipWithIndex.zip(fnArgs).map {
        case ((paramType, index), arg) =>
          paramType match {
            case cl if cl == classOf[Int] ⇒
              cast(
                arg,
                _.toInt,
                s"Arg $index of '$arg' not an int " +
                  s"(or passed string argument isn't matched the corresponding argument in Wasm function)"
              )
            case cl if cl == classOf[Long] ⇒
              cast(
                arg,
                _.toLong,
                s"Arg $index of '$arg' not a long " +
                  s"(or passed string argument isn't matched the corresponding argument in Wasm function)"
              )
            case cl if cl == classOf[Float] ⇒
              cast(
                arg,
                _.toFloat,
                s"Arg $index of '$arg' not a float " +
                  s"(or passed string argument isn't matched the corresponding argument in Wasm function)"
              )
            case cl if cl == classOf[Double] ⇒
              cast(
                arg,
                _.toDouble,
                s"Arg $index of '$arg' not a double " +
                  s"(or passed string argument isn't matched the corresponding argument in Wasm function"
              )
            case _ ⇒
              Left(
                InvalidArgError(
                  s"Invalid type for param $index: $paramType, expected [i32|i64|f32|f64]"
                )
              )
          }
      }

      result ← list2Either[F, InvalidArgError, AnyRef](args.toList)

    } yield result

  }

}
