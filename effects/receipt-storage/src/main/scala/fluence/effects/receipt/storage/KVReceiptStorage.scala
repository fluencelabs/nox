package fluence.effects.receipt.storage

import java.nio.file.Path

import cats.data.EitherT
import cats.effect.{ContextShift, LiftIO, Resource, Sync}
import cats.syntax.flatMap._
import fluence.codec
import fluence.codec.{CodecError, PureCodec}
import fluence.effects.kvstore.{KVStore, RocksDBStore}
import fluence.effects.tendermint.block.history.Receipt
import cats.syntax.either._
import fluence.log.Log

import scala.language.higherKinds
import scala.util.Try

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

  implicit val stringCodec: PureCodec[String, Array[Byte]] = PureCodec.liftEitherB[String, Array[Byte]](
    str ⇒ Try(str.getBytes()).toEither.leftMap(t ⇒ CodecError("Cannot serialize string to bytes", Some(t))),
    bs ⇒ Try(new String(bs)).toEither.leftMap(t ⇒ CodecError("Cannot parse bytes to string", Some(t)))
  )

  implicit val longStringCodec: PureCodec[Long, String] = PureCodec.liftEitherB[Long, String](
    _.toString.asRight,
    str ⇒ Try(str.toLong).toEither.leftMap(t ⇒ CodecError("Cannot parse string to long", Some(t)))
  )

  implicit val longCodec: PureCodec[Long, Array[Byte]] =
    PureCodec[Long, String] >>> PureCodec[String, Array[Byte]]

  def make[F[_]: Sync: LiftIO: ContextShift: Log](appId: Long, storagePath: Path): Resource[F, KVReceiptStorage[F]] =
    for {
      path <- Resource.liftF(Sync[F].catchNonFatal(storagePath.resolve(ReceiptStoragePath).resolve(appId.toString)))
      store <- RocksDBStore.make[F, Long, Receipt](path.toAbsolutePath.toString)
    } yield new KVReceiptStorage[F](appId, store)
}
