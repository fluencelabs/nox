import cats.effect.concurrent.Ref
import cats.effect.{ContextShift, Fiber, IO, Timer}
import cats.syntax.apply._

import scala.concurrent.duration._

case class Order(ref: Ref[IO, Int])(implicit cs: ContextShift[IO], t: Timer[IO]) {
  def wait(id: Int): IO[Unit] = for {
    last <- ref.get
    _ <- if (id - last == 1) IO.unit else {
      IO.shift *> IO.sleep(100.millis) *> wait(id)
    }
  } yield ()
}
