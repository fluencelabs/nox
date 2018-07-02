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

package fluence.grpc

import java.io.ByteArrayInputStream

import cats.effect.IO
import fluence.grpc.GrpcMonix.{observerToStream, streamToObserver}
import fluence.stream.StreamHandler
import io.grpc.{CallOptions, ManagedChannel, MethodDescriptor}
import io.grpc.MethodDescriptor.MethodType
import io.grpc.internal.IoUtils
import io.grpc.stub.ClientCalls
import monix.execution.Scheduler
import monix.reactive.{MulticastStrategy, Observable, OverflowStrategy}

class GrpcHandler(
  private val serviceManager: ServiceManager,
  channelOptionsIO: IO[(ManagedChannel, CallOptions)]
)(implicit sch: Scheduler)
    extends StreamHandler {

  private val overflow: OverflowStrategy.Synchronous[Nothing] = OverflowStrategy.Unbounded

  private def checkType(methodDescriptor: MethodDescriptor[Any, Any], methodType: MethodType): IO[Unit] = {
    if (methodDescriptor.getType == methodType)
      IO.unit
    else
      IO.raiseError(
        new IllegalArgumentException(
          s"Cannot handle request, because method: ${methodDescriptor.getFullMethodName} is not of type: $methodType."
        )
      )
  }

  override def handle(
    service: String,
    method: String,
    requests: Observable[Array[Byte]]
  ): IO[Observable[Array[Byte]]] = {
    for {
      methodDescriptor ← serviceManager.getMethodDescriptorF[IO](service, method)
      _ ← checkType(methodDescriptor, MethodType.BIDI_STREAMING)
      channelOptions ← channelOptionsIO
      responses ← {
        IO {
          val (channel, callOptions) = channelOptions
          val call = channel.newCall[Any, Any](methodDescriptor, callOptions)

          val (inResp, outResp) = Observable.multicast[Any](MulticastStrategy.publish, overflow)
          val reqStreamObserver = ClientCalls.asyncBidiStreamingCall(call, observerToStream(inResp))
          val reqObserver = streamToObserver(reqStreamObserver)

          requests.map(r ⇒ methodDescriptor.parseRequest(new ByteArrayInputStream(r))).subscribe(reqObserver)

          val mappedOut = outResp.map { r ⇒
            IoUtils.toByteArray(methodDescriptor.streamResponse(r))
          }

          mappedOut
        }
      }
    } yield responses
  }

  override def handleUnary(service: String, method: String, request: Array[Byte]): IO[Array[Byte]] = {
    for {
      methodDescriptor ← serviceManager.getMethodDescriptorF[IO](service, method)
      _ ← checkType(methodDescriptor, MethodType.UNARY)
      channelOptions ← channelOptionsIO
      responseObservable ← IO {
        val (channel, callOptions) = channelOptions
        val call = channel.newCall[Any, Any](methodDescriptor, CallOptions.DEFAULT)

        val (inResp, outResp) = Observable.multicast[Any](MulticastStrategy.replay, overflow)

        ClientCalls.asyncUnaryCall(
          call,
          methodDescriptor.parseRequest(new ByteArrayInputStream(request)),
          observerToStream(inResp)
        )

        outResp.map { r ⇒
          IoUtils.toByteArray(methodDescriptor.streamResponse(r))
        }
      }
      response ← responseObservable.headL.toIO
    } yield response
  }

}
