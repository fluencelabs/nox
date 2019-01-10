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

package fluence.node.tendermint.json

import io.circe.Encoder
import io.circe.generic.semiauto.deriveEncoder

/**
 * Information about the cluster without node-specific data.
 *
 * @param genesis genesis in Tendermint format
 * @param persistent_peers p2p peers in Tendermint format
 * @param external_addrs external address used to initialize advertised address in launching scripts
 */
case class Cluster(genesis: Genesis, persistent_peers: String, external_addrs: Seq[String])

object Cluster {
  implicit val clusterEncoder: Encoder[Cluster] = deriveEncoder[Cluster]
}
