package fluence

trait Default[A] {
  def default: A
}

object Default {
  def apply[A](implicit instance: Default[A]): Default[A] = instance

  implicit def defaultForString: Default[String] =
    new Default[String] {
      override def default: String = "0000"
    }
}
