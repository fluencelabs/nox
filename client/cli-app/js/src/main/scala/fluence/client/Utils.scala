package fluence.client

object Utils {
  def prettyResult(result: Option[String]) = s"`${result.getOrElse("null")}`"
}
