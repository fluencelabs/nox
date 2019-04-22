package fluence.merkle

trait ConvertsTo[A, B] {
  def convert(v: A): B
}

object ConvertsTo {
  def apply[A, B](implicit instance: ConvertsTo[A, B]): ConvertsTo[A, B] = instance

  implicit def convertIdentity[A]: ConvertsTo[A, A] =
    new ConvertsTo[A, A] {
      override def convert(v: A): A = v
    }
}
