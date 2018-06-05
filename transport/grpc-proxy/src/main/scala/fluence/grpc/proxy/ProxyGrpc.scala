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
import io.grpc.stub.ClientCalls
import monix.eval.{MVar, Task}
import monix.execution.Scheduler.Implicits.global
import monix.reactive.{MulticastStrategy, Observable, Observer, OverflowStrategy}

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

  import fluence.grpc.GrpcMonix._

  //TODO add auto-cleanup for old expired invoices
  val callCache: Task[MVar[Map[Long, Observer.Sync[Any]]]] = MVar(Map.empty[Long, Observer.Sync[Any]]).memoize

  private val overflow: OverflowStrategy.Synchronous[Nothing] = OverflowStrategy.Unbounded

  /**
   * Gets grpc method descriptor from registered services.
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
    Task(getMethodDescriptor(service, method)).flatMap {
      case Some(md) ⇒ Task.pure(md)
      case None ⇒ Task.raiseError(new IllegalArgumentException(s"There is no $service/$method method."))
    }

  /**
   * Creates listener for client call and connects it with observable.
   *
   */
  private def openBidiCall(
    methodDescriptor: MethodDescriptor[Any, Any]
  ): Task[(Observer.Sync[Any], Observable[Array[Byte]])] = {
    Task {

      val call = inProcessGrpc.newCall[Any, Any](methodDescriptor, CallOptions.DEFAULT)

      val (inResp, outResp) = Observable.multicast[Any](MulticastStrategy.replay, overflow)
      val reqStreamObserver = ClientCalls.asyncBidiStreamingCall(call, observerToStream(inResp))
      val reqObserver = streamToObserver(reqStreamObserver)

      val mappedOut = outResp.map { r ⇒
        IoUtils.toByteArray(methodDescriptor.streamResponse(r))
      }

      (reqObserver, mappedOut)
    }
  }

  /**
   * Creates observable, sends single request and closes stream on proxy side.
   * The observable and the call will close automatically when the response returns.
   */
  private def handleUnaryCall(
    reqE: Either[StatusException, Any],
    methodDescriptor: MethodDescriptor[Any, Any]
  ): Task[Observable[Array[Byte]]] = {
    Task {

      val call = inProcessGrpc.newCall[Any, Any](methodDescriptor, CallOptions.DEFAULT)

      val (inResp, outResp) = Observable.multicast[Any](MulticastStrategy.replay, overflow)
      reqE match {
        case Right(req) ⇒
          ClientCalls.asyncUnaryCall(call, req, observerToStream(inResp))

          outResp.map { r ⇒
            IoUtils.toByteArray(methodDescriptor.streamResponse(r))
          }
        case Left(ex) ⇒
          Observable.raiseError[Array[Byte]](ex)
      }
    }
  }

  private def call(obs: Observer.Sync[Any], reqE: Either[StatusException, Any]) = {
    reqE match {
      case Right(req) ⇒ obs.onNext(req)
      case Left(ex) ⇒ obs.onError(ex)
    }
  }

  /**
   * If it is new requestId, creates call and observable, that connects proxy server with proxy client and sends first message.
   * For requests that are cached we use an already created call, that connected with client through observable.
   */
  private def handleStreamCall(
    req: Either[StatusException, Any],
    methodDescriptor: MethodDescriptor[Any, Any],
    requestId: Long
  ): Task[Option[Observable[Array[Byte]]]] = {
    for {
      callOp ← callCache.flatMap(_.read).map(_.get(requestId))
      resp ← callOp match {
        case Some(c) ⇒
          Task {
            call(c, req)
            None
          }
        case None ⇒
          for {
            callWithObs ← openBidiCall(methodDescriptor)
            (c, obs) = callWithObs
            map ← callCache.flatMap(_.take)
            _ ← callCache.flatMap(_.put(map + (requestId -> c)))
          } yield {
            call(c, req)
            Some(obs)
          }
      }
    } yield resp
  }

  /**
   * Handles proxying request for some service and method that registered in grpc server.
   *
   * @param service Name of grpc service (class name of service).
   * @param method Name of grpc method (method name of service).
   * @param reqE Input stream of bytes or grpc error.
   *
   * @return Response as array of bytes or None, if there is no response.
   */
  def handleMessage(
    service: String,
    method: String,
    requestId: Long,
    reqE: Either[StatusException, InputStream]
  ): Task[Option[Observable[Array[Byte]]]] = {
    for {
      methodDescriptor ← getMethodDescriptorF(service, method)
      _ = logger.debug("Websocket method descriptor: " + methodDescriptor.toString)
      req ← Task(reqE.map(methodDescriptor.parseRequest))
      _ = logger.debug("Websocket request: " + req)
      resp ← {
        if (methodDescriptor.getType == MethodType.UNARY)
          handleUnaryCall(req, methodDescriptor).map(Some.apply)
        else
          handleStreamCall(req, methodDescriptor, requestId)
      }
    } yield resp
  }

}
