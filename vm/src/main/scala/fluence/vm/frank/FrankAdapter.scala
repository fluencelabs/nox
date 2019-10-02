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

package fluence.vm.frank
import fluence.vm.config.VmConfig
import fluence.vm.frank.result.{RawInitializationResult, RawInvocationResult, RawStateComputationResult}
import ch.jodersky.jni.nativeLoader

/**
 * Realizes connection to the virtual machine runner based on Wasmer through JNI.
 */
@nativeLoader("frank")
class FrankAdapter {

  /**
   * Initializes execution environment with given file path.
   *
   * @param filePath path to a wasm file
   */
  @native def initialize(filePath: String, config: VmConfig): RawInitializationResult

  /**
   * Invokes main module handler.
   *
   * @param arg argument for invoked module
   */
  @native def invoke(arg: Array[Byte]): RawInvocationResult

  /**
   * Returns hash of all significant inner state of the VM.
   */
  @native def computeVmState(): RawStateComputationResult
}
