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

package fluence.effects.tendermint.block.history.db

import java.nio.file.{Files, Path}

import cats.data.EitherT
import cats.effect.{ContextShift, LiftIO, Resource, Sync, Timer}
import cats.instances.either._
import cats.instances.list._
import cats.syntax.applicativeError._
import cats.syntax.either._
import cats.syntax.flatMap._
import cats.syntax.apply._
import cats.syntax.applicative._
import cats.syntax.foldable._
import cats.syntax.functor._
import cats.{Defer, Monad, MonadError, Traverse}
import fluence.effects.EffectError
import fluence.effects.kvstore.{KVStore, RocksDBStore}
import fluence.effects.tendermint.block.data
import fluence.effects.tendermint.block.history.db.Blockstore.rocksDbStore
import fluence.effects.tendermint.block.protobuf.{Protobuf, ProtobufConverter}
import fluence.log.Log
import io.circe.parser.parse
import org.rocksdb.{RocksDBException, Status}
import proto3.tendermint.{Block, BlockMeta, BlockPart}

import scala.collection.JavaConverters._
import scala.concurrent.duration._
import scala.language.higherKinds

trait Blockstore[F[_]] {

  /**
   * Retrieves block at the given height from Tendermint's database
   * @param height Block height
   * @return Block
   */
  def getBlock(height: Long): EitherT[F, BlockstoreError, data.Block]

  /**
   * Retrieves current height of the cluster from Tendermint's database
   */
  def getStorageHeight: EitherT[F, BlockstoreError, Long]
}

object Blockstore {
  type RawKV[F[_]] = KVStore[F, Array[Byte], Array[Byte]]

  // Tendermint stores height of the last stored block at this key
  val BlockStoreHeightKey: Array[Byte] = "blockStore".getBytes

  /**
   * Creates symlinks for all files with .sst extension, "renaming" them to .ldb extension.
   *
   * This is needed because Tendermint uses LevelDB (it requires .ldb), but our KVStore uses RocksDB (requires .sst).
   * So in order to make RocksDB read LevelDB files, this hack is used. Also, see this issue on why RocksDB doesn't support
   * .ldb files https://github.com/facebook/rocksdb/issues/677#issuecomment-523069054
   */
  private def createSymlinks[F[_]: Log](
    levelDbDir: Path
  )(implicit F: Sync[F]) = {
    import Files.{createSymbolicLink => createSymlink}

    def ldbToSst(file: Path) = file.getFileName.toString.replaceFirst(".ldb$", ".sst")
    def ls(dir: Path) = Files.list(dir).iterator().asScala.toSeq
    def rmDir(dir: Path) = (ls(dir) :+ dir).foreach(Files.delete)
    def makeSymlinks(link: Path, target: Path) = ls(target).foreach(f => createSymlink(link.resolve(ldbToSst(f)), f))

    Resource.make(
      F.delay {
        // TODO: use constant file name
        val tmpDir = Files.createTempDirectory("leveldb_rocksdb")
        val dbDir = levelDbDir.toAbsolutePath
        makeSymlinks(tmpDir, dbDir)
        tmpDir
      }.attempt.map(_.leftMap {
        case e: BlockstoreError => e
        case e                  => SymlinkCreationError(e, levelDbDir)
      })
    )(p => F.delay(p.foreach(rmDir)).attempt.void)
  }

  private def rocksDbStore[F[_]: Log: Monad: LiftIO: ContextShift: Defer](p: Path): Resource[F, Blockstore[F]] =
    RocksDBStore
      .makeRaw[F](p.toString, createIfMissing = false, readOnly = true)
      .map(kv => new BlockstoreImpl(kv))

  // TODO: using MonadError here because caller (DockerWorkerServices) uses it, avoid doing that
  private def raiseLeft[F[_]: Log: Sync, T, E <: Throwable](
    r: Resource[F, Either[E, T]],
    tendermintPath: Path
  ): Resource[F, T] =
    r.evalMap {
      case Left(e) =>
        Log[F].error(s"Error on creating blockstore for $tendermintPath") >>
          Sync[F].raiseError[T](e)
      case Right(b) => Log[F].trace(s"Blockstore created for $tendermintPath").as(b)
    }

  def make[F[_]: Sync: LiftIO: ContextShift: Timer](
    tendermintPath: Path
  )(implicit log: Log[F]): Resource[F, Blockstore[F]] =
    log.scope("blockstore") { implicit log: Log[F] =>
      raiseLeft(
        Monad[Resource[F, ?]].tailRecM(tendermintPath.resolve("data").resolve("blockstore.db")) { path =>
          val storeOrError = for {
            dbPath <- createSymlinks[F](path)
            _ <- Log.resource[F].debug(s"Opening DB at $dbPath")
            store <- Traverse[Either[BlockstoreError, ?]].sequence(dbPath.map(rocksDbStore[F]))
          } yield store

          storeOrError.evalMap {
            case Left(e: RocksDBException) if Option(e.getStatus).exists(_.getCode == Status.Code.NotFound) =>
              Log[F]
                .warn("Not all symlinks were created – creating them again", e)
                .as(path.asLeft[Either[Throwable, Blockstore[F]]])

            case Left(e: RocksDBException) if Option(e.getStatus).isEmpty =>
              Log[F]
                .warn("RocksDBException with empty status – trying again", e)
                .as(
                  path.asLeft[Either[Throwable, Blockstore[F]]]
                )

            case Left(e @ SymlinkCreationError(_: java.nio.file.NoSuchFileException, _)) =>
              Log[F]
                .warn("Tendermint isn't initialized yet – sleeping 5 sec & trying again", e) >>
                Timer[F]
                  .sleep(5.seconds)
                  .as(path.asLeft[Either[Throwable, Blockstore[F]]])

            case Left(e) =>
              Log[F]
                .warn("Error while creating store – raising", e)
                .as((e: Throwable).asLeft[Blockstore[F]].asRight[Path])

            case Right(store) =>
              Log[F].info("Store created").as(store.asRight[Throwable].asRight[Path])
          }
        },
        tendermintPath
      )
    }
}
