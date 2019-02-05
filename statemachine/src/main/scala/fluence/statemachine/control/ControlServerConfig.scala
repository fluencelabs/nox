package fluence.statemachine.control

/** Settings for [[ControlServer]]
 * @param host host to listen on
 * @param port port to listen on
 */
case class ControlServerConfig(host: String, port: Short)
