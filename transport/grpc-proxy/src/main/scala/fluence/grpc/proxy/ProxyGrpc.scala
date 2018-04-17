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

package fluence.grpc.proxy

import java.io.InputStream

import io.grpc.MethodDescriptor.MethodType
import io.grpc._
import io.grpc.internal.IoUtils
import monix.eval.{MVar, Task}
import monix.execution.Scheduler.Implicits.global
import monix.reactive.{MulticastStrategy, Observable, OverflowStrategy}

import scala.concurrent.ExecutionContext
import scala.language.higherKinds

/**
 * Service to proxy requests in grpc from another source.
 *
 * @param inProcessGrpc In-process services and client channel.
 */
class ProxyGrpc(inProcessGrpc: InProcessGrpc)(
  implicit ec: ExecutionContext
) extends slogging.LazyLogging {

  /**
   * Get grpc method descriptor from registered services.
   *
   * @param service Name of service.
   * @param method Name of method.
   *
   * @return Method descriptor or None, if there is no descriptor in registered services.
   */
  private def getMethodDescriptor(service: String, method: String): Option[MethodDescriptor[Any, Any]] = {
    for {
      serviceDescriptor ← inProcessGrpc.services.find(_.getServiceDescriptor.getName == service)
      serverMethodDefinition ← Option(
        serviceDescriptor.getMethod(service + "/" + method).asInstanceOf[ServerMethodDefinition[Any, Any]]
      )
      methodDescriptor ← Option(serverMethodDefinition.getMethodDescriptor)
    } yield methodDescriptor
  }

  private def getMethodDescriptorF(service: String, method: String): Task[MethodDescriptor[Any, Any]] =
    Task.eval(getMethodDescriptor(service, method)).flatMap {
      case Some(md) ⇒ Task.pure(md)
      case None ⇒ Task.raiseError(new IllegalArgumentException(s"There is no $service/$method method."))
    }

  val callCache: Task[MVar[Map[Long, ClientCall[Any, Any]]]] = MVar(Map.empty[Long, ClientCall[Any, Any]]).memoize

  private val overflow: OverflowStrategy.Synchronous[Nothing] = OverflowStrategy.Unbounded

  def openBidiCall(
    methodDescriptor: MethodDescriptor[Any, Any]
  ): Task[(ClientCall[Any, Any], Observable[Response])] = {
    Task.eval {
      val metadata = new Metadata()
      val call = inProcessGrpc.newCall[Any, Any](methodDescriptor, CallOptions.DEFAULT)

      val (in, out) = Observable.multicast[Any](MulticastStrategy.replay, overflow)

      call.start(new StreamProxyListener[Any](in), metadata)

      val mappedOut = out.collect {
        case r ⇒
          ResponseArrayByte(IoUtils.toByteArray(methodDescriptor.streamResponse(r))): Response
      }

      (call, mappedOut)
    }
  }

  /**
   * Handle proxying request for some service and method that registered in grpc server.
   *
   * @param service Name of grpc service (class name of service).
   * @param method Name of grpc method (method name of service).
   * @param stream Input stream of bytes.
   *
   * @return Response as array of bytes.
   */
  def handleMessage(
    service: String,
    method: String,
    streamId: Long,
    stream: InputStream
  ): Task[Observable[Response]] = {
    for {
      methodDescriptor ← getMethodDescriptorF(service, method)

      callOp ← callCache.flatMap(_.read).map(_.get(streamId))
      req ← Task.eval(methodDescriptor.parseRequest(stream))
      resp ← callOp match {
        case Some(c) ⇒
          Task.eval {
            c.sendMessage(req)
            c.request(1)
            Observable()
          }
        case None ⇒
          for {
            callWithObs ← openBidiCall(methodDescriptor)
            (c, obs) = callWithObs
            _ ← if (methodDescriptor.getType != MethodType.UNARY) {
              for {
                map ← callCache.flatMap(_.take)
                _ ← callCache.flatMap(_.put(map + (streamId -> c)))
              } yield ()
            } else Task.unit
          } yield {
            c.sendMessage(req)
            c.request(1)

            if (methodDescriptor.getType == MethodType.UNARY) c.halfClose()
            obs
          }
      }

    } yield resp
  }

}

sealed trait Response
final case class ResponseArrayByte(bytes: Array[Byte]) extends Response
