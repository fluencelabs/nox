package fluence.node

import cats.data.NonEmptyList
import cats.effect.concurrent.Ref
import cats.effect.{ContextShift, IO, Timer}
import fluence.log.{Log, LogFactory}
import fluence.node.workers.WorkersApi
import fluence.node.workers.subscription.{RequestSubscriber, ResponsePromise}

import scala.concurrent.ExecutionContext.Implicits.global

class RequestResponseSpec {

  def startSpec() = {

    implicit val ioTimer: Timer[IO] = IO.timer(global)
    implicit val ioShift: ContextShift[IO] = IO.contextShift(global)
    implicit val log = LogFactory.forPrintln[IO]().init("AbciServiceSpec", level = Log.Error).unsafeRunSync()

    val ref = Ref.unsafe[IO, Map[Long, NonEmptyList[ResponsePromise[IO]]]](
      Map.empty[Long, NonEmptyList[ResponsePromise[IO]]]
    )
    val requestSubscriber = RequestSubscriber(ref)

    val pool = TestWorkersPool.some[IO].unsafeRunSync()

    val call = WorkersApi.txWaitResponse[IO, IO.Par](pool, requestSubscriber, 1, "", None)
  }
}
