package fluence.effects.tendermint.block.history.db

import cats.data.EitherT
import proto3.tendermint.Block

import scala.language.higherKinds

trait TendermintBlockstore[F[_]] {
  def getBlock(height: Long): EitherT[F, BlockstoreError, Block]
  def getStorageHeight: EitherT[F, BlockstoreError, Int]
}
