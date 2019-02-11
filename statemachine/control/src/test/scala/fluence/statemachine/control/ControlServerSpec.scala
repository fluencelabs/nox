package fluence.statemachine.control
import cats.effect.{ContextShift, IO, Resource, Timer}
import cats.implicits._
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend
import fluence.statemachine.control.ControlServer.ControlServerConfig
import io.circe.Encoder
import org.scalatest.{EitherValues, Matchers, OptionValues, WordSpec}
import scodec.bits.ByteVector
import cats.syntax.either._

import scala.concurrent.ExecutionContext.Implicits.global

trait ControlServerOps extends EitherValues with OptionValues {
  import com.softwaremill.sttp.circe._
  import com.softwaremill.sttp.{SttpBackend, _}

  val config: ControlServerConfig

  def send[Req: Encoder](request: Req, path: String)(
    implicit b: SttpBackend[IO, Nothing]
  ): IO[Response[String]] = {
    sttp
      .body(request)
      .post(uri"http://${config.host}:${config.port}/control/$path")
      .send()
  }
}

class ControlServerSpec extends WordSpec with Matchers with ControlServerOps {
  val config = ControlServerConfig("localhost", 26662)

  "ControlServer" should {
    implicit val ioTimer: Timer[IO] = IO.timer(global)
    implicit val ioShift: ContextShift[IO] = IO.contextShift(global)

    val server = ControlServer.make[IO](config)
    val sttp = Resource.make(IO(AsyncHttpClientCatsBackend[IO]()))(sttpBackend ⇒ IO(sttpBackend.close()))
    val resources = server.flatMap(srv => sttp.map(srv -> _))

    "respond with 404" in {
      resources.use {
        case (_, sttp) =>
          implicit val sttpBackend = sttp
          for {
            response <- send("", "wrongPath")
          } yield {
            response.code shouldBe 404
          }
      }.unsafeRunSync()
    }

    "receive DropPeer event" in {
      resources.use {
        case (server, sttp) =>
          implicit val sttpBackend = sttp
          for {
            cp <- IO.pure(DropPeer(ByteVector.fill(32)(1)))
            response <- send[DropPeer](cp, "dropPeer")
            received <- server.signals.dropPeers.use(IO.pure)
          } yield {
            response.code shouldBe 200
            response.body.right.value shouldBe ""

            received.length shouldBe 1
            received.head shouldBe cp
          }
      }.unsafeRunSync()
    }

    "receive several ChangePeer events" in {
      val count = 3
      resources.use {
        case (server, sttp) =>
          implicit val sttpBackend = sttp
          for {
            dp <- IO.pure(DropPeer(ByteVector.fill(32)(1)))
            dps = Array.fill(count)(dp).toList
            _ <- dps.map(send(_, "dropPeer")).sequence
            received <- server.signals.dropPeers.use(IO.pure)
          } yield {

            received.length shouldBe 3
            received.foreach { r =>
              r shouldBe dp
            }
          }
      }.unsafeRunSync()
    }
  }
}
