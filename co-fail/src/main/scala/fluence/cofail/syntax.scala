package fluence.cofail

import cats.Monad
import cats.syntax.flatMap._
import cats.data.EitherT
import shapeless._

import scala.language.higherKinds

object syntax {

  // TODO: it must not make a coproduct from A <: AA or A >: AA, it should merge them into supertype instead
  implicit class EitherTSyntax[F[_], A, B](self: EitherT[F, A, B]) {
    def flatMap[AA, D](f: B ⇒ EitherT[F, AA, D])(implicit
                                                 F: Monad[F],
                                                 evLeftNotMatch: AA =:!= A): EitherT[F, AA :+: A :+: CNil, D] =
      EitherT[F, AA :+: A :+: CNil, D](self.value.flatMap {
        case Right(r) ⇒
          f(r).leftMap(Coproduct[AA :+: A :+: CNil](_)).value
        case Left(l) ⇒
          F.pure(Left(Coproduct[AA :+: A :+: CNil](l)))
      })
  }

}
