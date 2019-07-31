package fluence.effects.receipt.storage

import java.nio.ByteBuffer
import java.nio.file.Path

import cats.data.EitherT
import cats.effect.{Concurrent, ContextShift, LiftIO, Resource, Sync}
import cats.syntax.flatMap._
import fluence.codec
import fluence.codec.{CodecError, PureCodec}
import fluence.effects.kvstore.{KVStore, MVarKVStore, RocksDBStore}
import fluence.effects.tendermint.block.history.Receipt
import cats.syntax.either._
import fluence.log.Log

import scala.language.higherKinds

/**
 * Implementation of ReceiptStorage with KVStore
 */
class KVReceiptStorage[F[_]: Sync](val appId: Long, store: KVStore[F, Long, Receipt]) extends ReceiptStorage[F] {

  /**
   * Stores receipt for the specified app at a given height
   */
  override def put(height: Long, receipt: Receipt)(implicit log: Log[F]): EitherT[F, ReceiptStorageError, Unit] =
    store.put(height, receipt).leftMap(PutError(appId, height, _))

  /**
   * Gets a receipt for specified app and height
   */
  override def get(height: Long)(implicit log: Log[F]): EitherT[F, ReceiptStorageError, Option[Receipt]] =
    store.get(height).leftMap(GetError(appId, height, _))

  /**
   * Retrieves a chain of receipts, starting at block height `from`, until `to`
   */
  override def retrieve(
    from: Option[Long],
    to: Option[Long]
  )(implicit log: Log[F]): fs2.Stream[F, (Long, Receipt)] = {

    val stream = store.stream
    val dropped = from.fold(stream)(from => stream.dropWhile(_._1 < from))

    to.fold(dropped)(to => dropped.takeWhile(_._1 < to))
  }
}

object KVReceiptStorage {

  private val ReceiptStoragePath = "receipt-storage"

  private implicit val receiptCodec: codec.PureCodec[Array[Byte], Receipt] =
    codec.PureCodec.liftEitherB(
      Receipt.fromBytesCompact(_).leftMap(e => CodecError("deserializing receipt via fromBytesCompact", Some(e))),
      _.bytesCompact().asRight
    )

  private implicit val longBytesCodec: PureCodec[Long, Array[Byte]] =
    PureCodec.liftB(ByteBuffer.allocate(8).putLong(_).array(), ByteBuffer.wrap(_).getLong)

  /**
   * Makes a persistent, RocksDB-backed ReceiptStorage
   *
   * @param appId Application ID
   * @param storagePath Data is stored in storagePath/`ReceiptStoragePath`/`appId`
   */
  def make[F[_]: Sync: LiftIO: ContextShift: Log](appId: Long, storagePath: Path): Resource[F, ReceiptStorage[F]] =
    for {
      path <- Resource.liftF(Sync[F].catchNonFatal(storagePath.resolve(ReceiptStoragePath).resolve(appId.toString)))
      store <- RocksDBStore.make[F, Long, Receipt](path.toAbsolutePath.toString)
    } yield new KVReceiptStorage[F](appId, store)

  /**
   * Makes an in-memory (not persistent) ReceiptStorage, suitable for testing
   *
   * @param appId Application ID
   */
  def makeInMemory[F[_]: Concurrent](appId: Long): Resource[F, ReceiptStorage[F]] =
    MVarKVStore.make[F, Long, Receipt]().map(s ⇒ new KVReceiptStorage[F](appId, s))
}
