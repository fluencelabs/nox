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

package fluence.node

import java.nio.file.{Files, Path, Paths}

import cats.effect._
import fluence.node.docker.{DockerIO, DockerImage, DockerParams}

import scala.language.higherKinds

trait DockerSetup extends OsSetup {
  protected val dockerHost: String = getOS match {
    case "linux" => ifaceIP("docker0")
    case "mac" => "host.docker.internal"
    case os => throw new RuntimeException(s"$os isn't supported")
  }

  protected val ethereumHost: String = getOS match {
    case "linux" => linuxHostIP.get
    case "mac" => "host.docker.internal"
    case os => throw new RuntimeException(s"$os isn't supported")
  }

  private def tempDirectory[F[_]: Sync]: Resource[F, String] = {
    // Java gives /var/folder, Docker requires it to be /private/var/folder
    // https://docs.docker.com/docker-for-mac/osxfs/#namespaces
    def macOsHack(path: String): String = getOS match {
      case "mac" => path.replaceFirst("^/var/folder", "/private/var/folder")
      case _ => path
    }

    Resource.make(
      Sync[F]
        .delay(macOsHack(Files.createTempDirectory("testvolume").toString))
    )(
      tempPath => Sync[F].delay(Paths.get(tempPath).toFile.delete())
    )
  }

  protected def runMaster[F[_]: ContextShift: Async](
    portFrom: Short,
    portTo: Short,
    name: String,
    apiPort: Short
  ): Resource[F, String] =
    tempDirectory.flatMap { masterDir =>
      DockerIO
        .run[F](
          DockerParams
            .build()
            .option("-e", s"TENDERMINT_IP=$dockerHost")
            .option("-e", s"ETHEREUM_IP=$ethereumHost")
            .option("-e", s"MIN_PORT=$portFrom")
            .option("-e", s"MAX_PORT=$portTo")
            .option("-e", s"SWARM_ENABLED=false")
            .port(apiPort, 5678)
            .option("--name", name)
            .volume(masterDir, "/master")
            .volume("/var/run/docker.sock", "/var/run/docker.sock")
            // statemachine expects wasm binaries in /vmcode folder
            .volume(
              // TODO: by defaults, user.dir in sbt points to a submodule directory while in Idea to the project root
              System.getProperty("user.dir")
                + "/../vm/examples/llamadb/target/wasm32-unknown-unknown/release",
              "/master/vmcode/vmcode-llamadb"
            )
            .image(DockerImage("fluencelabs/node", "latest"))
            .daemonRun(),
          20
        )
        .map(_.containerId)
    }
}
