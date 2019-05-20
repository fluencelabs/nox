package fluence.node.workers.tendermint

import cats.Monad
import cats.effect.{Concurrent, Resource}
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.history.Receipt
import fluence.node.workers.Worker

import scala.language.higherKinds

object BlockUploading extends slogging.LazyLogging {

  def start[F[_]: Concurrent](worker: Worker[F]): Resource[F, Unit] = {
    val services = worker.services
    def upload(block: Block): F[Receipt] = ???

    Resource
      .make(
        Concurrent[F].start(
          services.blocks.evalMap { blockRaw =>
            Block(blockRaw) match {
              case Left(e) => Monad[F].pure(logger.error(s"DWS failed to parse Tendermint block: ${e.getMessage}"))
              case Right(block) => upload(block) >>= services.control.sendBlockReceipt
            }
          }.compile.drain
        )
      )(_.cancel)
      .void
  }
}
