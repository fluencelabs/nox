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

package fluence.swarm

/**
 * Swarm URL scheme. List and description of schemes:
 * https://swarm-guide.readthedocs.io/en/latest/usage/bzz.html#bzz-protocol-suite
 *
 */
sealed abstract class BzzProtocol(val protocol: String)

object BzzProtocol {

  // The bzz scheme assumes that the domain part of the url points to a manifest.
  case object Bzz extends BzzProtocol("bzz:")

  //  Allows you to receive hash pointers to content
  // that the ENS (https://swarm-guide.readthedocs.io/en/latest/usage.html#using-ens-names) entry resolved to at different versions
  case object BzzResource extends BzzProtocol("bzz-resource:")

  // Get resource directly by hash (possible to get raw manifest)/
  case object BzzRaw extends BzzProtocol("bzz-raw:")
}
