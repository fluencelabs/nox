package fluence.dataset.grpc.server

import cats.~>
import fluence.codec.Codec
import fluence.dataset.grpc.{Contract, ContractAllocatorGrpc}
import fluence.dataset.protocol.ContractAllocatorRpc

import scala.concurrent.Future
import scala.language.higherKinds

class ContractAllocatorServer[F[_], C](contractAllocator: ContractAllocatorRpc[F, C])
                                      (implicit codec: Codec[F, C, Contract],
                                       run: F ~> Future)
  extends ContractAllocatorGrpc.ContractAllocator {

  override def offer(request: Contract): Future[Contract] =
    run(
      for {
        c <- codec.decode(request)
        offered <- contractAllocator.offer(c)
        resp <- codec.encode(offered)
      } yield resp
    )

  override def allocate(request: Contract): Future[Contract] =
    run(
      for {
        c <- codec.decode(request)
        allocated <- contractAllocator.allocate(c)
        resp <- codec.encode(allocated)
      } yield resp
    )
}
