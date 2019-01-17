/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.node
import java.io.File
import java.net.InetAddress

import cats.effect.{ContextShift, IO, Sync, Timer}
import cats.syntax.applicativeError._
import cats.syntax.functor._
import cats.syntax.monadError._
import org.scalactic.source.Position
import org.scalatest.exceptions.{TestFailedDueToTimeoutException, TestFailedException}
import org.scalatest.time.Span
import org.scalatest.{BeforeAndAfterAll, OptionValues, Suite}
import slogging.LazyLogging

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration.FiniteDuration
import scala.io.Source
import scala.sys.process.{Process, ProcessLogger}
import scala.util.Try
import scala.concurrent.duration._
import scala.language.higherKinds

trait Integration extends LazyLogging with BeforeAndAfterAll with OptionValues {
  self: Suite ⇒

  implicit protected val ioTimer: Timer[IO] = IO.timer(global)
  implicit protected val ioShift: ContextShift[IO] = IO.contextShift(global)

  val bootstrapDir = new File("../bootstrap")
  private def run(cmd: String): Unit = Process(cmd, bootstrapDir).!(ProcessLogger(_ => ()))
  private def runBackground(cmd: String): Unit = Process(cmd, bootstrapDir).run(ProcessLogger(_ => ()))

  protected val dockerHost = getOS match {
    case "linux" => ifaceIP("docker0")
    case "mac" => "host.docker.internal"
    case os => throw new RuntimeException(s"$os isn't supported")
  }

  protected val ethereumHost = getOS match {
    case "linux" => linuxHostIP
    case "mac" => "host.docker.internal"
    case os => throw new RuntimeException(s"$os isn't supported")
  }

  override def beforeAll(): Unit = {
    // TODO: It is needed to build vm-llamadb project explicitly for launch this test from Idea
    logger.info("bootstrapping npm")
    run("npm install")

    logger.info("starting Ganache")
    runBackground("npm run ganache")

    logger.info("deploying contract to Ganache")
    run("npm run migrate")
  }

  override def afterAll(): Unit = {
    logger.info("killing ganache")
    run("pkill -f ganache")

    logger.info("stopping containers")
    // TODO: kill containers through Master's HTTP API
    run("docker rm -f 01_node0 01_node1 master1 master2")
  }

  // return IP address of the `interface`
  private def ifaceIP(interface: String): String = {
    import sys.process._
    val ifconfigCmd = Seq("ifconfig", interface)
    val grepCmd = Seq("grep", "inet ")
    val awkCmd = Seq("awk", "{print $2}")
    InetAddress.getByName((ifconfigCmd #| grepCmd #| awkCmd).!!.replaceAll("[^0-9\\.]", "")).getHostAddress
  }

  private def linuxHostIP = {
    import sys.process._
    val ipR = "(?<=src )[0-9\\.]+".r
    ipR.findFirstIn("ip route get 8.8.8.8".!!.trim).value
  }

  private def getOS: String = {
    // TODO: should use more comprehensive and reliable OS detection
    val osName = System.getProperty("os.name").toLowerCase()
    if (osName.contains("windows"))
      "windows"
    else if (osName.contains("mac") || osName.contains("darwin"))
      "mac"
    else
      "linux"
  }

  protected def eventually[F[_]: Sync: Timer](
    p: => F[Unit],
    period: FiniteDuration = 1.second,
    maxWait: FiniteDuration = 10.seconds
  )(implicit pos: Position): F[_] = {
    fs2.Stream
      .awakeEvery[F](period)
      .take((maxWait / period).toLong)
      .evalMap(_ => p.attempt)
      .takeThrough(_.isLeft) // until p returns Right(Unit)
      .compile
      .last
      .map {
        case Some(Right(_)) =>
        case Some(Left(e)) => throw e
        case _ => throw new RuntimeException(s"eventually timed out after $maxWait")
      }
      .adaptError {
        case e: TestFailedException =>
          e.modifyMessage(m => Some(s"eventually timed out after $maxWait" + m.map(": " + _).getOrElse("")))
        case e =>
          new TestFailedDueToTimeoutException(
            _ => Some(s"eventually timed out after $maxWait" + Option(e.getMessage).map(": " + _).getOrElse("")),
            Some(e),
            pos,
            None,
            Span.convertDurationToSpan(maxWait)
          )
      }
  }

  protected def heightFromTendermintStatus(startPort: Int): IO[Option[Long]] = IO {
    import io.circe._
    import io.circe.parser._
    val port = startPort + 100 // +100 corresponds to port mapping scheme from `ClusterData`
    val source = Source.fromURL(s"http://localhost:$port/status").mkString
    val height = parse(source)
      .getOrElse(Json.Null)
      .asObject
      .flatMap(_("result"))
      .flatMap(_.asObject)
      .flatMap(_("sync_info"))
      .flatMap(_.asObject)
      .flatMap(_("latest_block_height"))
      .flatMap(_.asString)
      .flatMap(x => Try(x.toLong).toOption)
    height
  }
}
