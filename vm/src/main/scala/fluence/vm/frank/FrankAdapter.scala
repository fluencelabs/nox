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
import fluence.vm.InvocationResult
import fluence.vm.config.VmConfig

/**
 * Realizes connection to virtual machine runner based on Wasmer.
 */
class FrankAdapter {

  /**
   * Initializes execution environment with given file path.
   *
   * @param filePath path to a wasm file
   */
  @native def instantiate(filePath: String, config: VmConfig): Int

  /**
   * Invokes main module.
   *
   * @param arg argument for invoked module
   */
  @native def invoke(arg: Array[Byte]): Array[Byte]

  /**
   * Returns hash of all significant inner state of this VM. This function calculates
   * hashes for the state of each module and then concatenates them together.
   */
  @native def getVmState(): Array[Byte]
}
