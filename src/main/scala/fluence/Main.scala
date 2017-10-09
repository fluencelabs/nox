package fluence

import freestyle._
import freestyle.implicits._
import fluence.core.AppModule
import scala.concurrent.ExecutionContext.Implicits.global

import scala.concurrent.Future

object Main extends App {

  import cats.implicits._
  // import cats.implicits._

  import scala.concurrent.duration.Duration
  // import scala.concurrent.duration.Duration

  import scala.concurrent.Await
  // import scala.concurrent.Await

  import FutureHandlers._

  val futureValue = Program.program[AppModule.Op].interpret[Future]
  // Give me something with at least 3 chars and a number on it
  // futureValue: scala.concurrent.Future[Unit] = Future(<not completed>)

  Await.result(futureValue, Duration.Inf)

  println("hello world")
}