package fluence

import fluence.core.{ Interaction, Validate }

import scala.concurrent.ExecutionContext.Implicits.global
// import scala.concurrent.ExecutionContext.Implicits.global

import scala.concurrent.Future
// import scala.concurrent.Future

object FutureHandlers {

  implicit val validationHandler = new Validate.Handler[Future] {
    override def minSize(s: String, n: Int): Future[Boolean] = Future(s.size >= n)
    override def hasNumber(s: String): Future[Boolean] = Future(s.exists(c ⇒ "0123456789".contains(c)))
  }
  // validationHandler: Validation.Handler[scala.concurrent.Future] = $anon$1@3011acde

  implicit val interactionHandler = new Interaction.Handler[Future] {
    override def tell(s: String): Future[Unit] = Future.successful(println(s))
    override def ask(s: String): Future[String] = Future.successful { println(s); "This could have been user input 1" }
  }

}
