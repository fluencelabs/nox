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

package fluence.vm.frank.result

/**
 * Represents raw JNI result of FrankAdapter::computeVmState invoking.
 *
 * @param error represent various initialization errors, None - no error occurred
 * @param state computed state of Frank VM, valid only if no error occurred (error == None)
 */
final case class RawStateComputationResult(error: Option[String], state: Array[Byte])
