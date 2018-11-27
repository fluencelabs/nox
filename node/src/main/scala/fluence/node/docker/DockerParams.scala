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

package fluence.node.docker
import scala.collection.immutable.Queue
import scala.sys.process._

/**
 * Builder for basic `docker run` command parameters.
 *
 * @param params Current command' params
 * @param tailParams Params to be added after the image name
 */
case class DockerParams private (params: Queue[String], tailParams: List[String] = Nil) {

  /**
   * Adds a single param to command.
   *
   * @param param option
   */
  def add(param: String): DockerParams =
    copy(params.enqueue(param))

  /**
   * Adds a named option to command.
   *
   * @param optionName option name
   * @param optionValue option value
   */
  def option(optionName: String, optionValue: String): DockerParams =
    add(optionName).add(optionValue)

  /**
   * Adds a port mapping.
   *
   * @param hostPort port number on host
   * @param containerPort mapped port number in container
   */
  def port(hostPort: Short, containerPort: Short): DockerParams =
    option("-p", s"$hostPort:$containerPort")

  /**
   * Adds a volume mapping.
   *
   * @param hostVolume volume directory on host
   * @param containerVolume mounted volume location in container
   */
  def volume(hostVolume: String, containerVolume: String): DockerParams =
    option("-v", s"$hostVolume:$containerVolume")

  def uid(uid: String): DockerParams =
    option("--user", uid)

  /**
   * Builds the current command to a representation ready to pass in [[scala.sys.process.Process]].
   *
   * @param imageName name of image to run
   */
  def image(imageName: String): DockerParams.Sealed =
    DockerParams.Sealed(add(imageName).params.enqueue(tailParams))
}

object DockerParams {
  case class Sealed(command: Seq[String]) extends AnyVal {
    def process: ProcessBuilder = Process(command)
  }

  def daemonRun(): DockerParams =
    DockerParams(Queue("docker", "run", "-d"))

  def run(executable: String, params: String*): DockerParams =
    DockerParams(Queue("docker", "run", "--user", "", "--rm", "-i", "--entrypoint", executable), params.toList)

}
