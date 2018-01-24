package fluence.node

import fluence.client.ClientComposer
import cats.instances.future._
import cats.~>
import fluence.transport.grpc.client.GrpcClient
import org.scalatest.{ BeforeAndAfterAll, Matchers, WordSpec }
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.time.{ Milliseconds, Seconds, Span }
import monix.execution.Scheduler.Implicits.global

import scala.concurrent.Future
import scala.language.higherKinds

class NodeComposerSpec extends WordSpec with Matchers with ScalaFutures with BeforeAndAfterAll {

  override implicit def patienceConfig: PatienceConfig = PatienceConfig(Span(10, Seconds), Span(250, Milliseconds))

  private implicit def runId[F[_]]: F ~> F = new (F ~> F) {
    override def apply[A](fa: F[A]): F[A] = fa
  }

  private val pureClient = ClientComposer.grpc[Future](GrpcClient.builder)


}
