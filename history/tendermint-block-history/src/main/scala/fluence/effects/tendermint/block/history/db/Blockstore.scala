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
import cats.effect.{ContextShift, LiftIO, Resource, Sync}
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
import proto3.tendermint.{Block, BlockMeta, BlockPart}

import scala.collection.JavaConverters._
import scala.language.higherKinds
import scala.util.Try

class Blockstore[F[_]: Log: Monad](kv: Blockstore.RawKV[F]) {
  import Blockstore._

  private def getOr[T](msg: String, height: Long)(opt: Option[T]): EitherT[F, BlockstoreError, T] =
    EitherT.fromOption(opt, GetBlockError(msg, height))

  private def metaKey(height: Long) = s"H:$height".getBytes()
  private def partKey(height: Long, index: Int) = s"P:$height:$index".getBytes()

  private def getBlockPartsCount(height: Long): EitherT[F, BlockstoreError, Int] =
    for {
      metaBytes <- kv
        .get(metaKey(height))
        .leftMap(e => GetBlockError(s"error getting block parts count: $e", height))
        .flatMap(getOr[Array[Byte]]("meta is none", height)(_))

      meta <- EitherT
        .fromEither[F](Protobuf.decode[BlockMeta](metaBytes))
        .leftMap(e => GetBlockError(s"error getting block parts count: $e", height))

      partsCount <- getOr[Int]("blockID.parts is none", height)(meta.blockID.flatMap(_.parts).map(_.total))
    } yield partsCount

  private def getPart(height: Long, i: Int): EitherT[F, BlockstoreError, BlockPart] =
    kv.get(partKey(height, i))
      .leftMap(e => GetBlockError(s"error retrieving block part $i from storage: $e", height))
      .flatMap(getOr[Array[Byte]](s"part $i not found", height))
      .subflatMap(Protobuf.decode[BlockPart])
      .leftMap(e => GetBlockError(s"error decoding block part $i from bytes: $e", height))

  private def loadParts(height: Long, count: Int): EitherT[F, BlockstoreError, Array[Byte]] =
    (0 until count).toList.foldM(Array.empty[Byte]) {
      case (bytes, idx) => getPart(height, idx).map(bytes ++ _.bytes.toByteArray)
    }

  private def decodeBlock(blockBytes: Array[Byte], height: Long): EitherT[F, BlockstoreError, Block] =
    EitherT
      .fromEither[F](Protobuf.decodeLengthPrefixed[Block](blockBytes))
      .leftMap(e => GetBlockError(s"error decoding block from bytes: $e", height))

  private def decodeHeight(heightJsonBytes: Array[Byte]): EitherT[F, BlockstoreError, Long] =
    EitherT
      .fromEither[F](
        Try(new String(heightJsonBytes)).toEither >>= parse >>= (_.hcursor.get[Long]("height"))
      )
      .leftMap(RetrievingStorageHeightError(_))

  private def getStorageHeightBytes =
    kv.get(BlockStoreHeightKey)
      .leftMap(RetrievingStorageHeightError(_))
      .flatMap(EitherT.fromOption(_, RetrievingStorageHeightError("blockStore height wasn't found")))

  private def convertBlock(block: Block): EitherT[F, BlockstoreError, data.Block] =
    EitherT
      .fromEither[F](ProtobufConverter.fromProtobuf(block))
      .leftMap(e => GetBlockError(s"Unable to convert block from protobuf: $e", block.header.fold(-1L)(_.height)))

  def getBlock(height: Long): EitherT[F, BlockstoreError, data.Block] =
    for {
      count <- getBlockPartsCount(height)
      bytes <- loadParts(height, count)
      pBlock <- decodeBlock(bytes, height)
      block <- convertBlock(pBlock)
    } yield block

  def getStorageHeight: EitherT[F, BlockstoreError, Long] =
    for {
      bytes <- getStorageHeightBytes
      height <- decodeHeight(bytes)
    } yield height
}

object Blockstore {
  type RawKV[F[_]] = KVStore[F, Array[Byte], Array[Byte]]

  val BlockStoreHeightKey: Array[Byte] = "blockStore".getBytes

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

  private def rocksDbStore[F[_]: Log: Monad: LiftIO: ContextShift: Defer](p: Path) =
    RocksDBStore
      .makeRaw[F](p.toString, createIfMissing = false, readOnly = true)
      .map(kv => new Blockstore(kv))

  def make[F[_]: Log: Sync: LiftIO: ContextShift](tendermintPath: Path): Resource[F, Blockstore[F]] =
    (for {
      dbPath <- createSymlinks[F](tendermintPath.resolve("data").resolve("blockstore.db"))
      _ <- Log.resource[F].debug(s"Opening DB at $dbPath")
      store <- Traverse[Either[BlockstoreError, ?]].sequence(dbPath.map(rocksDbStore))
    } yield store).evalMap {
      // TODO: using MonadError here because caller–DockerWorkerServices–uses it, avoid doing that
      case Left(e)  => Sync[F].raiseError(e)
      case Right(b) => b.pure[F]
    }
}
