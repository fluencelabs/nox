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
import fluence.stream.Connection
import io.grpc.MethodDescriptor.MethodType
import io.grpc.internal.IoUtils
import io.grpc.stub.{ClientCalls, StreamObserver}
import io.grpc.{CallOptions, ManagedChannel, MethodDescriptor}
import monix.execution.Scheduler
import monix.reactive.{Observable, OverflowStrategy}

class GrpcConnection(
  private val serviceManager: ServiceManager,
  channelOptionsIO: IO[(ManagedChannel, CallOptions)]
)(implicit sch: Scheduler)
    extends Connection {

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

          val operator = { obs: StreamObserver[Any] ⇒
            ClientCalls.asyncBidiStreamingCall(call, obs)
          }

          //TODO delete double serialization
          val requestsProto = requests.map(r ⇒ methodDescriptor.parseRequest(new ByteArrayInputStream(r)))

          GrpcMonix.liftByGrpcOperator(requestsProto, operator).map { r ⇒
            IoUtils.toByteArray(methodDescriptor.streamResponse(r))
          }
        }
      }
    } yield responses
  }

  override def handleUnary(service: String, method: String, request: Array[Byte]): IO[Array[Byte]] = {
    for {
      methodDescriptor ← serviceManager.getMethodDescriptorF[IO](service, method)
      _ ← checkType(methodDescriptor, MethodType.UNARY)
      channelOptions ← channelOptionsIO
      response ← {
        val (channel, callOptions) = channelOptions
        val call = channel.newCall[Any, Any](methodDescriptor, callOptions)

        GrpcMonix
          .guavaFutureToIO(
            ClientCalls.futureUnaryCall(
              call,
              methodDescriptor.parseRequest(new ByteArrayInputStream(request))
            )
          )
          .map { r ⇒
            IoUtils.toByteArray(methodDescriptor.streamResponse(r))
          }
      }
    } yield response
  }

}

object GrpcConnection {
  def builder(serviceManager: ServiceManager)(channelOptions: IO[(ManagedChannel, CallOptions)])(implicit scheduler: Scheduler): GrpcConnection = {
    new GrpcConnection(serviceManager, channelOptions)
  }
}
