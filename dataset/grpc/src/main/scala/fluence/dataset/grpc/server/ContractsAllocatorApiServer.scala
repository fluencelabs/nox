package fluence.dataset.grpc.server

import java.nio.ByteBuffer

import cats.~>
import fluence.codec.Codec
import fluence.dataset.grpc.{Contract, DatasetContractsApiGrpc, FindRequest}
import fluence.dataset.protocol.ContractsAllocatorApi
import fluence.kad.protocol.Key
import io.grpc.stub.StreamObserver

import scala.concurrent.Future
import scala.language.higherKinds

class ContractsAllocatorApiServer[F[_], C](
                                            api: ContractsAllocatorApi[F, C]
                                          )(implicit codec: Codec[F, C, Contract],
                                            keyCodec: Codec[F, Key, ByteBuffer],
                                            run: F ~> Future)
  extends DatasetContractsApiGrpc.DatasetContractsApi {

  // TODO: implement
  override def allocate(responseObserver: StreamObserver[Contract]): StreamObserver[Contract] =
    new StreamObserver[Contract] {
      override def onError(t: Throwable): Unit = ???

      override def onCompleted(): Unit = ???

      override def onNext(value: Contract): Unit = ???
    }

  override def find(request: FindRequest): Future[Contract] =
    run(
      for {
        k <- keyCodec.decode(request.id.asReadOnlyByteBuffer())
        contract <- api.find(k)
        resp <- codec.encode(contract)
      } yield resp
    )
}
