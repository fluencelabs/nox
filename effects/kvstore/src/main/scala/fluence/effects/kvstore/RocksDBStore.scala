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

import cats.{~>, Defer, Monad}
import cats.data.EitherT
import cats.effect.{IO, LiftIO, Resource}
import org.rocksdb.{Options, ReadOptions, RocksDB, RocksIterator}
import cats.syntax.applicativeError._
import fluence.codec.PureCodec
import slogging.LazyLogging

import scala.language.higherKinds
import scala.util.control.NonFatal

object RocksDBStore extends LazyLogging {
  private def ioToF[F[_]: LiftIO]: IO ~> F = new (IO ~> F) {
    override def apply[A](fa: IO[A]): F[A] = fa.to[F]
  }

  /**
   * Makes RocksDB KVStore with user-friendly key and value types, taking codecs into account.
   *
   * @param folder Folder to store RockDB data, MUST be unique, cannot be used by different RocksDB instances simultaneously
   * @param createIfMissing Ask RocksDB to create data folder if it's missing
   * @param keysCodec Used to serialize/deserialize keys
   * @param valuesCodec Used to serialize/deserialize values
   * @tparam F Defer for Resource, LiftIO for IO
   * @tparam K Keys type
   * @tparam V Values type
   */
  def make[F[_]: Monad: Defer: LiftIO, K, V](
    folder: String,
    createIfMissing: Boolean = true
  )(
    implicit keysCodec: PureCodec[K, Array[Byte]],
    valuesCodec: PureCodec[Array[Byte], V]
  ): Resource[F, KVStore[F, K, V]] =
    makeRaw[F](folder, createIfMissing)
      .map(_.transform[K, V])

  /**
   * RocksDB inner type for keys and values is Array[Byte], so implement [[KVStore]] over it.
   *
   * @param folder Folder to store RockDB data, MUST be unique, cannot be used by different RocksDB instances simultaneously
   * @param createIfMissing Ask RocksDB to create data folder if it's missing
   * @tparam F Defer for Resource, LiftIO for IO
   */
  def makeRaw[F[_]: Monad: Defer: LiftIO](
    folder: String,
    createIfMissing: Boolean = true
  ): Resource[F, KVStore[F, Array[Byte], Array[Byte]]] =
    // We want to prepare all the C++ objects of RocksDB, and have all of them closed even in case of error
    (for {
      // Database options
      opts ← Resource.make(IO {
        logger.trace(s"Creating options")
        val options = new Options()
        logger.trace(s"Here we have options: " + options)
        options.setCreateIfMissing(createIfMissing)
        logger.trace(s"With a flag: " + options)
        options
      })(
        opts ⇒
          IO(opts.close()).handleError {
            case NonFatal(err) ⇒
              err.printStackTrace()
              logger.error(s"Cannot close Options object during cleanup: $err", err)
        }
      )

      _ = logger.trace("Created opts...")

      // Database itself
      data ← Resource.make(IO {
        RocksDB.loadLibrary()
        val dataDir = new File(folder)
        if (!dataDir.exists()) dataDir.mkdirs()
        RocksDB.open(opts, folder)
      })(
        data ⇒
          IO(data.close()).handleError {
            case NonFatal(err) ⇒
              err.printStackTrace()
              logger.error(s"Cannot close RocksDB object during cleanup: $err", err)
        }
      )

      _ = logger.trace("Created rocksdb...")

      // Read options -- could be used for optimizations later, e.g. snapshots
      readOptions ← Resource.make(IO(new ReadOptions()))(
        rOpts ⇒
          IO(rOpts.close()).handleError {
            case NonFatal(err) ⇒
              err.printStackTrace()
              logger.error(s"Cannot close RocksDB object during cleanup: $err", err)
        }
      )

      _ = logger.debug("Created readOpts...")

    } yield
      new KVStore[F, Array[Byte], Array[Byte]] {
        override def get(key: Array[Byte]): EitherT[F, KVReadError, Option[Array[Byte]]] =
          IO(Option(data.get(readOptions, key))).attemptT
            .mapK(ioToF)
            .leftMap(IOExceptionError("Cannot get value for a key", _))

        override def put(
          key: Array[Byte],
          value: Array[Byte]
        ): EitherT[F, KVWriteError, Unit] =
          IO(data.put(key, value)).attemptT
            .mapK(ioToF)
            .leftMap(IOExceptionError("Cannot put value", _))

        override def remove(
          key: Array[Byte]
        ): EitherT[F, KVWriteError, Unit] =
          IO(data.delete(key)).attemptT
            .mapK(ioToF)
            .leftMap(IOExceptionError("Cannot remove value for a key", _))

        override def stream: fs2.Stream[F, (Array[Byte], Array[Byte])] =
          fs2.Stream
            .bracket(
              IO(data.newIterator(readOptions)).to[F]
            )(
              it ⇒
                // Iterator is a c++ structure, so it must be closed
                IO(it.close()).handleError {
                  case NonFatal(err) ⇒
                    err.printStackTrace()
                    logger.error(s"Cannot close RocksDB iterator: $err", err)
                }.to[F]
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
      }).mapK(ioToF)
}
