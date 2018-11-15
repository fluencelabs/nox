package fluence.ethclient.helpers

/**
 * Builder for basic `docker run` command parameters.
 *
 * @param command current command
 */
case class DockerRunBuilder(private val command: Vector[String]) {

  /**
   * Adds a single option to command.
   *
   * @param option option
   */
  def add(option: String): DockerRunBuilder =
    new DockerRunBuilder(command :+ option)

  /**
   * Adds a named option to command.
   *
   * @param optionName option name
   * @param optionValue option value
   */
  def add(optionName: String, optionValue: String): DockerRunBuilder =
    new DockerRunBuilder(command :+ optionName :+ optionValue)

  /**
   * Adds a port mapping.
   *
   * @param hostPort port number on host
   * @param containerPort mapped port number in container
   */
  def addPort(hostPort: Short, containerPort: Short): DockerRunBuilder =
    add("-p", s"$hostPort:$containerPort")

  /**
   * Adds a volume mapping.
   *
   * @param hostVolume volume directory on host
   * @param containerVolume mounted volume location in container
   */
  def addVolume(hostVolume: String, containerVolume: String): DockerRunBuilder =
    add("-v", s"$hostVolume:$containerVolume")

  /**
   * Builds the current command to a representation ready to pass in [[scala.sys.process.Process]].
   *
   * @param imageName name of image to run
   */
  def build(imageName: String): Vector[String] = add(imageName).command
}

object DockerRunBuilder {
  def apply(): DockerRunBuilder = new DockerRunBuilder(Vector("docker", "run"))
}
