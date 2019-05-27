package fluence.effects.receipt.storage

import cats.data.EitherT
import cats.effect.{ContextShift, IO, LiftIO, Resource}
import cats.{Defer, Monad}
import fluence.codec
import fluence.codec.PureCodec
import fluence.effects.kvstore.{KVStore, RocksDBStore}
import fluence.effects.tendermint.block.history.Receipt

import scala.language.higherKinds
import scala.reflect.io.Path

private[storage] case class ReceiptKey(appId: Long, height: Long)
private[storage] object ReceiptKey {
  import cats.syntax.compose._

  implicit val cdc: codec.PureCodec[ReceiptKey, Array[Byte]] =
    PureCodec.liftB[ReceiptKey, String](rk => s"${rk.appId}_${rk.height}", s => {
      val Array(id, h) = s.split('_')
      ReceiptKey(id.toLong, h.toLong)
    }) andThen PureCodec[String, Array[Byte]]
}

/**
 * Implementation of ReceiptStorage with KVStore
 */
class KVReceiptStorage[F[_]: Monad](store: KVStore[F, ReceiptKey, Receipt]) extends ReceiptStorage[F] {

  /**
   * Stores receipt for the specified app at a given height
   */
  override def put(appId: Long, height: Long, receipt: Receipt): EitherT[F, ReceiptStorageError, Unit] =
    store.put(ReceiptKey(appId, height), receipt).leftMap(PutError(appId, height, _))

  /**
   * Gets a receipt for specified app and height
   */
  override def get(appId: Long, height: Long): EitherT[F, ReceiptStorageError, Option[Receipt]] =
    store.get(ReceiptKey(appId, height)).leftMap(GetError(appId, height, _))

  /**
   * Retrieves a chain of receipts, starting at block height `from`, until `to`
   */
  override def retrieve(
    appId: Long,
    from: Option[Long],
    to: Option[Long]
  ): F[List[(Long, Receipt)]] = {

    val stream = store.stream
    val dropped = from.fold(stream)(from => stream.dropWhile(_._1.height < from))
    val taken = to.fold(dropped)(to => dropped.takeWhile(_._1.height < to))

    taken.compile.toList
  }
}

object KVReceiptStorage {
  import ReceiptKey._
  import cats.syntax.flatMap._

  private val ReceiptStoragePath = "receipt-storage"

  private implicit val receiptCodec: codec.PureCodec[Array[Byte], Receipt] =
    codec.PureCodec.liftB(Receipt.fromBytesCompact, _.bytesCompact())

  def make[F[_] : Monad : Defer : LiftIO : ContextShift](appId: Long, storagePath: Path): Resource[F, KVReceiptStorage[F]] =
    for {
      // TODO: handling errors via IO... Smells?
      path <- Resource.liftF(IO(storagePath.resolve(ReceiptStoragePath).resolve(appId.toString)).to[F])
      store <- RocksDBStore.make[F, ReceiptKey, Receipt](path.toAbsolute.toString())
    } yield new KVReceiptStorage[F](store)
}
