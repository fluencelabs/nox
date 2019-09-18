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

import cats.data.{EitherT, NonEmptyList}
import cats.effect.LiftIO
import cats.Monad
import com.typesafe.config.{Config, ConfigFactory}
import fluence.log.Log
import fluence.vm.VmError.WasmVmError.{ApplyError, GetVmStateError, InvokeError}
import fluence.vm.config.VmConfig
import fluence.vm.frank.{FrankAdapter, FrankWasmVm}
import scodec.bits.ByteVector

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

  /**
   * Temporary way to pass a flag from userland (the WASM file) to the Node, denotes whether an app
   * expects outer world to pass Ethereum blocks data into it.
   * TODO move this flag to the Smart Contract
   */
  val expectsEth: Boolean
}

object WasmVm {
  val javaLibPath: String = System.getProperty("java.library.path")
  System.out.println(javaLibPath)

  Runtime.getRuntime.load(
    "/Users/trofim/Desktop/work/fluence/fluence/vm/executor/target/release/libwasm_executor.dylib"
  )

  /**
   * Main method factory for building VM.
   * Compiles all files immediately by Asmble and returns VM implementation with eager module instantiation.
   *
   * @param inFiles input files in wasm or wast format
   * @param configNamespace a path of config in 'lightbend/config terms, please see reference.conf
   */
  def apply[F[_]: Monad: Log](
    inFiles: NonEmptyList[String],
    configNamespace: String = "fluence.vm.client",
    conf: ⇒ Config = ConfigFactory.load()
  ): EitherT[F, ApplyError, WasmVm] =
    for {
      // reading config
      config ← VmConfig.readT[F](configNamespace, conf)

      _ ← Log.eitherT[F, ApplyError].info("WasmVm: configs read...")
      vmRunnerInvoker = new FrankAdapter()

      _ = vmRunnerInvoker.instantiate(inFiles.head, config)
    } yield new FrankWasmVm(
      vmRunnerInvoker,
      config
    )
}
