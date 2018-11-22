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

package fluence.node.tendermint
import java.nio.file.{Files, Path}

import cats.effect.IO
import io.circe.Encoder
import io.circe.generic.semiauto._

/**
 * Information about the cluster and current node. Compatible with `master-run-node.sh` script.
 *
 * TODO: there should be no `master-run-node.sh` script
 *
 * @param cluster cluster information
 * @param node_index node index
 */
case class NodeInfo(cluster: Cluster, node_index: String) {
  def clusterName: String = cluster.genesis.chain_id

  def writeTo(tendermintPath: Path): IO[Unit] = IO {
    val configPath = tendermintPath.resolve("config")
    Files.createDirectories(configPath)
    Files.write(configPath.resolve("node_info.json"), NodeInfo.nodeInfoEncoder(this).spaces2.getBytes)
  }
}

object NodeInfo {
  implicit val nodeInfoEncoder: Encoder[NodeInfo] = deriveEncoder[NodeInfo]
}
