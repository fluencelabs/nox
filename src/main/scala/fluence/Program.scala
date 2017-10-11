package fluence

import freestyle._
import freestyle.implicits._
import fluence.core.AppModule
import scala.language.higherKinds

object Program {

  def program[F[_]](implicit A: AppModule[F]): FreeS[F, Unit] = {
    import A._
    import cats.implicits._

    for {
      userInput ← interaction.ask("Give me something with at least 3 chars and a number on it")
      valid ← (validation.minSize(userInput, 3) |@| validation.hasNumber(userInput)).map(_ && _).freeS
      _ ← if (valid)
        interaction.tell("awesomesauce!")
      else
        interaction.tell(s"$userInput is not valid")
    } yield ()
  }

}
