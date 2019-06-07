package fluence.effects.receipt.storage

import java.nio.ByteBuffer
import java.nio.file.Path

import cats.data.EitherT
import cats.effect.{ContextShift, LiftIO, Resource, Sync}
import cats.syntax.flatMap._
import fluence.codec
import fluence.codec.PureCodec
import fluence.effects.kvstore.{KVStore, RocksDBStore}
import fluence.effects.tendermint.block.history.Receipt

import scala.language.higherKinds

/**
 * Implementation of ReceiptStorage with KVStore
 */
class KVReceiptStorage[F[_]: Sync](val appId: Long, store: KVStore[F, Long, Receipt]) extends ReceiptStorage[F] {

  /**
   * Stores receipt for the specified app at a given height
   */
  override def put(height: Long, receipt: Receipt): EitherT[F, ReceiptStorageError, Unit] =
    store.put(height, receipt).leftMap(PutError(appId, height, _))

  /**
   * Gets a receipt for specified app and height
   */
  override def get(height: Long): EitherT[F, ReceiptStorageError, Option[Receipt]] =
    store.get(height).leftMap(GetError(appId, height, _))

  /**
   * Retrieves a chain of receipts, starting at block height `from`, until `to`
   */
  override def retrieve(
    from: Option[Long],
    to: Option[Long]
  ): fs2.Stream[F, (Long, Receipt)] = {

    val stream = store.stream
    val dropped = from.fold(stream)(from => stream.dropWhile(_._1 < from))

    to.fold(dropped)(to => dropped.takeWhile(_._1 < to))
  }
}

object KVReceiptStorage {
  import cats.syntax.compose._
  import cats.syntax.flatMap._

  private val ReceiptStoragePath = "receipt-storage"

  private implicit val receiptCodec: codec.PureCodec[Array[Byte], Receipt] =
    codec.PureCodec.liftB(Receipt.fromBytesCompact, _.bytesCompact())

  implicit val longBytesCodec: PureCodec[Long, Array[Byte]] =
    PureCodec.liftB(ByteBuffer.allocate(4).putLong(_).array(), ByteBuffer.wrap(_).getLong)

  def make[F[_]: Sync: LiftIO: ContextShift](appId: Long, storagePath: Path): Resource[F, KVReceiptStorage[F]] =
    for {
      path <- Resource.liftF(Sync[F].catchNonFatal(storagePath.resolve(ReceiptStoragePath).resolve(appId.toString)))
      store <- RocksDBStore.make[F, Long, Receipt](path.toAbsolutePath.toString)
    } yield new KVReceiptStorage[F](appId, store)
}
