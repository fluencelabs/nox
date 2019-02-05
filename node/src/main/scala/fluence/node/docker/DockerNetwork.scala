package fluence.node.docker

case class DockerNetwork(name: String) extends AnyVal

object DockerNetwork {
  def name(appId: Long, workerIdx: Int) = s"network_${appId}_$workerIdx"
}
