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

package fluence.statemachine.contract
import fluence.statemachine.util.ClientInfoMessages
import fluence.statemachine.{ClientId, PublicKey}

/**
 * Client registry stub.
 *
 * Currently supports only 2 known hardcoded (client, public key) pairs for now.
 */
class ClientRegistry {

  private val knownClients: Map[ClientId, PublicKey] = Map(
    "client001" -> "94QLUpF/i65eJTeThLF4w+xhhu4hHsEqHeO9h7na5Kw=",
    "client002" -> "JSzg1etjAeYNLaN7QwHgnPYF/0IlSdcmZmWplZJkutY="
  )

  /**
   * Returns public key used to verify transaction signed by given `client`.
   *
   * @param client client identifier
   * @return either client's [[PublicKey]] or error message
   */
  def getPublicKey(client: ClientId): Either[String, PublicKey] =
    Either.cond(knownClients.contains(client), knownClients(client), ClientInfoMessages.UnknownClient)
}
