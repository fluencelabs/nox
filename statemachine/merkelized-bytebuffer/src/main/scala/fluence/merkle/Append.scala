package fluence.merkle

trait Append[A] {
  def append(appendables: List[A]): A
}

object Append {
  def apply[A](implicit instance: Append[A]): Append[A] = instance

  implicit def appendString[A]: Append[String] =
    new Append[String] {
      override def append(appendables: List[String]): String = appendables.mkString("<>")
    }
}
