package fluence.node.workers.tendermint.block
import cats.Applicative
import cats.effect.Resource
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fluence.node.workers.Worker

import scala.language.higherKinds

/**
 * Block uploading that does nothing – used to disable block uploading
 */
class DisabledBlockUploading[F[_]: Applicative] extends BlockUploading[F] {

  /**
   * Subscribe on new blocks from tendermint and upload them one by one to the decentralized storage
   * For each block:
   *   1. retrieve vmHash from state machine
   *   2. Send block manifest receipt to state machine
   *
   * @param worker Blocks are coming from this worker's Tendermint; receipts are sent to this worker
   */
  override def start(worker: Worker[F])(implicit log: Log[F], backoff: Backoff[EffectError]): Resource[F, Unit] =
    Resource.pure(())
}
