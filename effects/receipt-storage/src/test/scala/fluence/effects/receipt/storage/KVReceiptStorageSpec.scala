package fluence.effects.receipt.storage

import java.nio.file.Files

import cats.effect.{ContextShift, IO, Timer}
import fluence.effects.tendermint.block.history.Receipt
import org.scalatest.{Matchers, OptionValues, WordSpec}
import scodec.bits.ByteVector
import cats.syntax.apply._
import fluence.log.{Log, LogFactory}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.language.postfixOps

class KVReceiptStorageSpec extends WordSpec with Matchers with OptionValues {
  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)

  implicit val log: Log[IO] = LogFactory.forPrintln[IO]().init("KVReceiptStorageSpec").unsafeRunSync()

  private val appId = 123L
  private val storagePath = Files.createTempDirectory("KVReceiptStorageSpec")
  private val storage = KVReceiptStorage.make[IO](appId, storagePath)

  "kv storage" should {
    "store and retrieve a receipt" in {
      val receipt = Receipt(ByteVector(1, 2, 3))

      val stored = storage.use { storage =>
        storage.put(1L, receipt) *> storage.get(1L) value
      }.unsafeRunSync()

      stored.isRight shouldBe true
      stored.right.get shouldBe defined
      stored.right.get.value shouldBe receipt
    }
  }
}
