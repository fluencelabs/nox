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
import fluence.node.docker.DockerParams.WithImage

import scala.collection.immutable.Queue
import scala.sys.process._

/**
 * Builder for basic `docker run` command parameters.
 *
 * @param params Current command' params
 */
case class DockerParams private (params: Queue[String]) {

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

  /**
   * Specifies a user on whose behalf commands will be run
   * @param user user login or uid
   */
  def user(user: String): DockerParams =
    option("--user", user)

  /**
   * Builds the current command to a representation ready to pass in [[scala.sys.process.Process]].
   *
   * @param imageName name of image to run
   */
  def image(imageName: String): DockerParams.WithImage =
    WithImage(params, imageName)
}

object DockerParams {
  sealed trait SealedParams {
    def command: Seq[String]
    def process: ProcessBuilder = Process(command)
  }
  case class DaemonParams(command: Seq[String]) extends SealedParams
  case class ExecParams(command: Seq[String]) extends SealedParams

  private val daemonParams = Seq("docker", "run", "-d")
  private val runParams = Seq("docker", "run", "--user", "", "--rm", "-i", "--entrypoint")

  case class WithImage(params: Seq[String], imageName: String) {

    def daemonRun(): DaemonParams =
      DaemonParams(daemonParams ++ params :+ imageName)

    def unmanagedDaemonRun(): ExecParams = {
      ExecParams(daemonParams ++ params :+ imageName)
    }

    def run(executable: String, execParams: String*): ExecParams = {
      val cmd = (runParams :+ executable) ++ params ++ (imageName +: execParams)
      ExecParams(cmd)
    }
  }

  def build(): DockerParams = DockerParams(Queue())
}
