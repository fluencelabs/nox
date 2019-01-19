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

package fluence.util
import java.io.File
import java.net.InetAddress

import cats.effect._
import cats.syntax.applicativeError._
import cats.syntax.functor._
import cats.syntax.monadError._
import fluence.node.docker.{DockerIO, DockerParams}
import org.scalactic.source.Position
import org.scalatest.exceptions.{TestFailedDueToTimeoutException, TestFailedException}
import org.scalatest.time.Span
import org.scalatest.{Timer => _}

import scala.concurrent.duration._
import scala.io.Source
import scala.language.higherKinds
import scala.sys.process.{Process, ProcessLogger}
import scala.util.Try

package object util {

  val bootstrapDir = new File("../bootstrap")
  def runCmd(cmd: String): Unit = Process(cmd, bootstrapDir).!(ProcessLogger(_ => ()))
  def runBackground(cmd: String): Unit = Process(cmd, bootstrapDir).run(ProcessLogger(_ => ()))

  val dockerHost: String = getOS match {
    case "linux" => ifaceIP("docker0")
    case "mac" => "host.docker.internal"
    case os => throw new RuntimeException(s"$os isn't supported")
  }

  val ethereumHost: String = getOS match {
    case "linux" => linuxHostIP.get
    case "mac" => "host.docker.internal"
    case os => throw new RuntimeException(s"$os isn't supported")
  }

  def eventually[F[_]: Sync: Timer](
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

  // return IP address of the `interface`
  def ifaceIP(interface: String): String = {
    import sys.process._
    val ifconfigCmd = Seq("ifconfig", interface)
    val grepCmd = Seq("grep", "inet ")
    val awkCmd = Seq("awk", "{print $2}")
    InetAddress.getByName((ifconfigCmd #| grepCmd #| awkCmd).!!.replaceAll("[^0-9\\.]", "")).getHostAddress
  }

  def linuxHostIP = {
    import sys.process._
    val ipR = "(?<=src )[0-9\\.]+".r
    ipR.findFirstIn("ip route get 8.8.8.8".!!.trim)
  }

  def getOS: String = {
    // TODO: should use more comprehensive and reliable OS detection
    val osName = System.getProperty("os.name").toLowerCase()
    if (osName.contains("windows"))
      "windows"
    else if (osName.contains("mac") || osName.contains("darwin"))
      "mac"
    else
      "linux"
  }

  def heightFromTendermintStatus(startPort: Int): IO[Option[Long]] = IO {
    import io.circe.parser.parse
    import io.circe.Json
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

  def runMaster[F[_]: ContextShift: Async](
    portFrom: Short,
    portTo: Short,
    name: String,
    statusPort: Short
  ): F[String] = {
    DockerIO
      .run[F](
        DockerParams
          .build()
          .option("-e", s"TENDERMINT_IP=$dockerHost")
          .option("-e", s"ETHEREUM_IP=$ethereumHost")
          .option("-e", s"PORTS=$portFrom:$portTo")
          .port(statusPort, 5678)
          .option("--name", name)
          .volume("/var/run/docker.sock", "/var/run/docker.sock")
          // statemachine expects wasm binaries in /vmcode folder
          .volume(
            // TODO: by defaults, user.dir in sbt points to a submodule directory while in Idea to the project root
            System.getProperty("user.dir")
              + "/../vm/examples/llamadb/target/wasm32-unknown-unknown/release",
            "/master/vmcode/vmcode-llamadb"
          )
          .image("fluencelabs/node:latest")
          .unmanagedDaemonRun()
      )
      .compile
      .lastOrError
  }
}
