/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.node.grpc

import java.io.{ByteArrayInputStream, InputStream, SequenceInputStream}

import cats.effect._
import fluence.transport.grpc.server.GrpcServer
import io.grpc._
import org.http4s.EntityEncoder._
import org.http4s._
import org.http4s.dsl.io._
import org.http4s.server.Server
import org.http4s.server.blaze.BlazeBuilder
import org.http4s.util.CaseInsensitiveString

import scala.collection.JavaConverters._
import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.Promise
import scala.util.Try

case class Result(resp: Any, headers: Metadata, status: io.grpc.Status, trailers: Metadata)

class HttpProxyHttp4s(server: GrpcServer) extends slogging.LazyLogging {

  val services = server.serverRef.get().getImmutableServices.asScala.toList

  val inp = new InProcessGrpc("in-process", services)

  val helloWorldService = HttpService[IO] {
    case req @ GET ⇒
      logger.error(req.toString())
      logger.error(s"Hello, get.")
      Ok(s"Hello, get.")
    case req @ POST -> Root / service / method ⇒
      logger.error(req.toString())
      logger.error(s"Hello, post.")
      logger.error(s"service is === " + service)
      logger.error(s"method is === " + method)
      val stringReq = req.as[Array[Byte]].unsafeRunSync()
      logger.error(s"Req as string === " + stringReq.mkString(","))

      val methodDescriptor = for {
        m ← inp.getMd[Any, Any](service, method)
      } yield {
        logger.error("CALL THIS!!!")

        val onMessagePr = Promise[Any]
        val onHeadersPr = Promise[Metadata]
        val onClosePr = Promise[(io.grpc.Status, Metadata)]
        Try {
          val md = m.getMethodDescriptor
          val marshalledReq =
            md.parseRequest(new ByteArrayInputStream(stringReq.slice(5, stringReq.length)))

          val metadata = new Metadata()
          val call = inp.chs.newCall[Any, Any](md, CallOptions.DEFAULT)

          call.start(new ProxyListener[Any](onMessagePr, onHeadersPr, onClosePr), metadata)
          call.sendMessage(marshalledReq)
          call.request(1)
          call.halfClose()

          onMessagePr.future.zip(onHeadersPr.future).zip(onClosePr.future).map {
            case ((res, headers), (status, trailers)) ⇒
              Result(res, headers, status, trailers)
          }
        }.recover {
          case e: Throwable ⇒
            e.printStackTrace()
            logger.error("SOME ERROR", e)
            onMessagePr.tryFailure(e)
            onMessagePr.future.zip(onHeadersPr.future).zip(onClosePr.future).map {
              case ((res, headers), (status, trailers)) ⇒
                Result(res, headers, status, trailers)
            }
        }.get
      }

      val a = for {
        res ← IO.fromFuture(IO(methodDescriptor.get)).attempt
        stat ← res match {
          case Right(r) ⇒
            println("RESPONSE === " + r)
            val m = inp.getMd[Any, Any](service, method).get
            println("11111")
            val md = m.getMethodDescriptor
            println("222222")
            val marshalledResp = md.getResponseMarshaller.stream(r.resp)

            val startStream = new ByteArrayInputStream(Array[Byte](0, 0, 0, 1, 35))

            val streams: InputStream = new SequenceInputStream(startStream, marshalledResp)

            println("333333")

            logger.error("We return some")
            val is = fs2.io.readInputStream(IO(streams), 100000, true)

            println("44444444")

            Ok("")
              .map(_.withBodyStream(is))
              .map(
                _.replaceAllHeaders(
                  req.headers
                    .filter(
                      p ⇒ p.name != CaseInsensitiveString("Host") && p.name != CaseInsensitiveString("Connection")
                    )
                    .put(Header("grpc-status", r.status.getCode.value().toString))
                    .put(Header("grpc-message", Option(r.status.getDescription).getOrElse("")))
                )
              )
          case Left(e) ⇒
            e.printStackTrace()
            InternalServerError(e.getLocalizedMessage)
        }
      } yield {

        stat
      }

      for {
        respE ← a.attempt
      } yield {
        respE match {
          case Left(e) ⇒
            e.printStackTrace()
          case _ ⇒
        }
      }
      a

    case req ⇒
      logger.error(req.toString())
      logger.error("hi rec")
      Ok("Hi req")
  }

}

object HttpProxyHttp4s {

  def builder(server: GrpcServer): IO[Server[IO]] = {
    val s = new HttpProxyHttp4s(server)

    BlazeBuilder[IO].bindHttp(8090).mountService(s.helloWorldService, "/").start
  }
}
