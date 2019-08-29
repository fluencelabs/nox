/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.effects.kvstore

import java.io.File
import java.util.concurrent.{ExecutorService, Executors}

import cats.data.EitherT
import cats.effect.{ContextShift, IO, LiftIO, Resource}
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{~>, Applicative, Defer, Monad}
import fluence.codec.PureCodec
import fluence.log.Log
import org.rocksdb.{Options, ReadOptions, RocksDB, RocksIterator}

import scala.concurrent.ExecutionContext
import scala.language.higherKinds
import scala.util.control.NonFatal

/**
 * RocksDB [[KVStore]] implementation.
 *
 * @param data RocksDB to operate with
 * @param readOptions RocksDB read options
 * @param ctx Execution context for RocksDB IO operations
 * @tparam F Effect
 */
class RocksDBStore[F[_]: Monad: LiftIO: ContextShift] private (
  data: RocksDB,
  readOptions: ReadOptions,
  ctx: ExecutionContext
) extends KVStore[F, Array[Byte], Array[Byte]] {

  private val ioToF = new (IO ~> F) {
    override def apply[A](fa: IO[A]): F[A] = ContextShift[F].evalOn(ctx)(fa.to[F])
  }

  override def get(key: Array[Byte])(implicit log: Log[F]): EitherT[F, KVReadError, Option[Array[Byte]]] =
    IO(Option(data.get(readOptions, key))).attemptT
      .mapK(ioToF)
      .leftMap(IOExceptionError("Cannot get value for a key", _))

  override def put(
    key: Array[Byte],
    value: Array[Byte]
  )(implicit log: Log[F]): EitherT[F, KVWriteError, Unit] =
    IO(data.put(key, value)).attemptT
      .mapK(ioToF)
      .leftMap(IOExceptionError("Cannot put value", _))

  override def remove(
    key: Array[Byte]
  )(implicit log: Log[F]): EitherT[F, KVWriteError, Unit] =
    IO(data.delete(key)).attemptT
      .mapK(ioToF)
      .leftMap(IOExceptionError("Cannot remove value for a key", _))

  override def stream(implicit log: Log[F]): fs2.Stream[F, (Array[Byte], Array[Byte])] =
    fs2.Stream
      .bracket(
        ioToF(IO(data.newIterator(readOptions)))
      )(
        it ⇒
          // Iterator is a c++ structure, so it must be closed
          ioToF(IO(it.close()).attempt).flatMap {
            case Left(NonFatal(err)) ⇒
              Log[F].error(s"Cannot close RocksDB iterator: $err", err)
            case Right(_) ⇒
              Applicative[F].unit
        }
      )
      .flatMap { it ⇒
        // Start iterator
        it.seekToFirst()
        // Unfold iterator to a stream
        fs2.Stream.unfold[F, RocksIterator, (Array[Byte], Array[Byte])](it)(
          iterator ⇒
            if (iterator.isValid) Some((iterator.key() -> iterator.value()) -> {
              iterator.next()
              iterator
            })
            else None
        )
      }
}

object RocksDBStore {

  /**
   * Makes RocksDB KVStore with user-friendly key and value types, taking codecs into account.
   *
   * @param folder Folder to store RockDB data, MUST be unique, cannot be used by different RocksDB instances simultaneously
   * @param createIfMissing Ask RocksDB to create data folder if it's missing
   * @param ex Executor service to build ExecutionContext for RocksDB operations
   * @param keysCodec Used to serialize/deserialize keys
   * @param valuesCodec Used to serialize/deserialize values
   * @tparam F Defer for Resource, LiftIO for IO, ContextShift to return execution back to the pool
   * @tparam K Keys type
   * @tparam V Values type
   */
  def make[F[_]: Monad: Defer: LiftIO: ContextShift: Log, K, V](
    folder: String,
    createIfMissing: Boolean = true,
    ex: ⇒ ExecutorService = Executors.newCachedThreadPool()
  )(
    implicit
    keysCodec: PureCodec[K, Array[Byte]],
    valuesCodec: PureCodec[Array[Byte], V]
  ): Resource[F, KVStore[F, K, V]] =
    makeRaw[F](folder, createIfMissing, ex)
      .map(_.transform[K, V])

  /**
   * RocksDB inner type for keys and values is Array[Byte], so implement [[KVStore]] over it.
   *
   * @param folder Folder to store RockDB data, MUST be unique, cannot be used by different RocksDB instances simultaneously
   * @param createIfMissing Ask RocksDB to create data folder if it's missing
   * @param ex Executor service to build ExecutionContext for RocksDB operations
   * @tparam F Defer for Resource, LiftIO for IO, ContextShift to return execution back to the pool
   */
  def makeRaw[F[_]: Monad: Defer: LiftIO: ContextShift: Log](
    folder: String,
    createIfMissing: Boolean = true,
    ex: ⇒ ExecutorService = Executors.newSingleThreadExecutor()
  ): Resource[F, KVStore[F, Array[Byte], Array[Byte]]] =
    // We want to prepare all the C++ objects of RocksDB, and have all of them closed even in case of error
    for {
      ctx ← Resource.make(IO(ExecutionContext.fromExecutorService(ex)).to[F])(
        ctx ⇒ IO(ctx.shutdown()).to[F]
      )

      _ ← Log.resource[F].trace("we have created ctx")

      cs = ContextShift[F]

      // Database options
      opts ← Resource.make(
        cs.evalOn(ctx)(
          for {
            _ ← Log[F].trace(s"Creating options")
            options ← IO(new Options()).to[F]
            _ ← Log[F].trace(s"Here we have options: " + options)
            _ ← IO(options.setCreateIfMissing(createIfMissing)).to[F]
            _ ← Log[F].trace(s"With a flag: " + options)
          } yield options
        )
      )(
        opts ⇒
          IO(opts.close()).attempt.to[F].flatMap {
            case Left(NonFatal(err)) ⇒
              Log[F].error(s"Cannot close Options object during cleanup", err)

            case Right(_) ⇒
              Applicative[F].unit
        }
      )

      _ ← Log.resource[F].trace("Created opts...")

      // Database itself
      data ← Resource.make(cs.evalOn(ctx)(IO {
        RocksDB.loadLibrary()
        val dataDir = new File(folder)
        if (!dataDir.exists()) dataDir.mkdirs()
        RocksDB.open(opts, folder)
      }.to[F]))(
        data ⇒
          IO(data.close()).attempt.to[F].flatMap {
            case Left(NonFatal(err)) ⇒
              Log[F].error(s"Cannot close RocksDB object during cleanup", err)
            case Right(_) ⇒
              Applicative[F].unit
        }
      )

      _ ← Log.resource[F].debug("Created rocksdb...")

      // Read options -- could be used for optimizations later, e.g. snapshots
      readOptions ← Resource.make(cs.evalOn(ctx)(IO(new ReadOptions()).to[F]))(
        rOpts ⇒
          IO(rOpts.close()).attempt.to[F].flatMap {
            case Left(NonFatal(err)) ⇒
              Log[F].error(s"Cannot close RocksDB object during cleanup", err)
            case Right(_) ⇒
              Applicative[F].unit
        }
      )

      _ ← Log.resource[F].trace("Created readOpts... going to return kvstore")

    } yield new RocksDBStore[F](data, readOptions, ctx): KVStore[F, Array[Byte], Array[Byte]]

}
