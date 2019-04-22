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

  implicit def appendBytes[A]: Append[Array[Byte]] =
    new Append[Array[Byte]] {
      override def append(appendables: List[Array[Byte]]): Array[Byte] = appendables.reduce(_ ++ _)
    }
}
