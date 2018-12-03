package fluence.node

case class SwarmConfig(protocol: String, host: String, port: Int, enabled: Boolean) {
  val addr: String = s"$protocol://$host:$port"
}
