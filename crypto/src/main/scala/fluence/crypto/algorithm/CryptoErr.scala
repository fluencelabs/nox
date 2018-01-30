package fluence.crypto.algorithm

case class CryptoErr(error: String, cause: Option[Throwable] = None) extends Throwable(error) {
  cause.foreach(initCause(_))
}
