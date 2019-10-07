package fluence.worker.responder

import cats.effect.{ContextShift, IO, Timer}
import fluence.bp.tx.Tx
import fluence.effects.{Backoff, EffectError}
import fluence.log.{Log, LogFactory}
import fluence.log.LogFactory.Aux
import fluence.log.appender.PrintlnLogAppender
import fluence.worker.responder.repeat.{SubscriptionKey, SubscriptionStorage}
import org.scalatest.{Matchers, OptionValues, WordSpec}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._

class SubscriptionStorageSpec extends WordSpec with OptionValues with Matchers {
  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)
  implicit private val backoff: Backoff[EffectError] = Backoff.default[EffectError]
  implicit private val logFactory: Aux[IO, PrintlnLogAppender[IO]] = LogFactory.forPrintln[IO](level = Log.Error)
  implicit private val log: Log.Aux[IO, PrintlnLogAppender[IO]] =
    logFactory.init("SubscriptionStorageSpec", level = Log.Off).unsafeRunTimed(5.seconds).value

  private def createStorageAndSubscription(): (SubscriptionStorage[IO], SubscriptionKey, Tx.Data) = {
    val storage = SubscriptionStorage[IO]().unsafeRunSync()
    val tx = "123"
    val txData = Tx.Data(tx.getBytes())
    val key = SubscriptionKey.generate("subscriptionId", txData)
    (storage, key, txData)
  }

  "subscription storage" should {
    "return false on adding subscription twice" in {
      val (storage, key, txData) = createStorageAndSubscription()
      storage.addSubscription(key, txData).unsafeRunSync() shouldBe true
      storage.addSubscription(key, txData).unsafeRunSync() shouldBe false
    }

    "return stream after adding subscription and stream" in {
      val (storage, key, txData) = createStorageAndSubscription()
      storage.addSubscription(key, txData).unsafeRunSync() shouldBe true
      storage.addStream(key, fs2.Stream.empty).unsafeRunSync()
      storage.getSubscriptions.unsafeRunSync().get(key).isDefined shouldBe true
    }

    "delete subscription on delete operation" in {
      val (storage, key, txData) = createStorageAndSubscription()
      storage.addSubscription(key, txData).unsafeRunSync() shouldBe true
      storage.deleteSubscription(key).unsafeRunSync()
      storage.getSubscriptions.unsafeRunSync().get(key).isEmpty shouldBe true
    }
  }
}
