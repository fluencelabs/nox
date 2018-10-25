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

package fluence.statemachine
import fluence.statemachine.util.Crypto
import slogging.LazyLogging

/**
 * Client with private key. Used for tests to signing transaction automatically.
 *
 * @param id client ID
 * @param privateKeyBase64 client private key
 */
case class SigningClient(id: String, privateKeyBase64: String) extends LazyLogging {

  /**
   * Signs a given `data` using private key.
   *
   * @param data text input
   * @return text Base-64 signature
   */
  def sign(data: String): String = Crypto.sign(data, privateKeyBase64)
}
