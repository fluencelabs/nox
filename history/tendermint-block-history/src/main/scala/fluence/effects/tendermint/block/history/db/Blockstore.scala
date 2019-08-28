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

import java.nio.file.{Files, Path, Paths}

import cats.data.EitherT
import cats.effect.{ContextShift, LiftIO, Resource, Sync}
import cats.instances.either._
import cats.instances.list._
import cats.syntax.applicativeError._
import cats.syntax.apply._
import cats.syntax.either._
import cats.syntax.flatMap._
import cats.syntax.foldable._
import cats.syntax.functor._
import cats.{Monad, Traverse}
import fluence.effects.kvstore.{KVStore, RocksDBStore}
import fluence.effects.tendermint.block.protobuf.Protobuf
import fluence.effects.{EffectError, WithCause}
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

  private def decodeHeight(heightJsonBytes: Array[Byte]): EitherT[F, BlockstoreError, Int] =
    EitherT
      .fromEither[F](
        Try(new String(heightJsonBytes)).toEither >>= parse >>= (_.hcursor.get[Int]("height"))
      )
      .leftMap(RetrievingStorageHeightError(_))

  private def getStorageHeightBytes =
    kv.get(BlockStoreHeightKey)
      .leftMap(RetrievingStorageHeightError(_))
      .flatMap(EitherT.fromOption(_, RetrievingStorageHeightError("blockStore height wasn't found")))

  def getBlock(height: Long): EitherT[F, BlockstoreError, Block] =
    for {
      count <- getBlockPartsCount(height)
      bytes <- loadParts(height, count)
      block <- decodeBlock(bytes, height)
    } yield block

  def getStorageHeight: EitherT[F, BlockstoreError, Int] =
    for {
      bytes <- getStorageHeightBytes
      height <- decodeHeight(bytes)
    } yield height
}

object Blockstore {
  type RawKV[F[_]] = KVStore[F, Array[Byte], Array[Byte]]

  val BlockStoreHeightKey: Array[Byte] = "blockStore".getBytes

  private def createSymlinks[F[_]: Log](
    levelDbDir: String
  )(implicit F: Sync[F]) = {
    import Files.{createSymbolicLink => createSymlink}

    def ldbToSst(file: Path) = file.getFileName.toString.replaceFirst(".ldb$", ".sst")
    def ls(dir: Path) = Files.list(dir).iterator().asScala.toSeq
    def rmDir(dir: Path) = (ls(dir) :+ dir).foreach(Files.delete)
    def makeSymlinks(link: Path, target: Path) = ls(target).foreach(f => createSymlink(link.resolve(ldbToSst(f)), f))

    Resource.make(
      F.delay {
        val tmpDir = Files.createTempDirectory("leveldb_rocksdb")
        val dbDir = Paths.get(levelDbDir).toAbsolutePath
        makeSymlinks(tmpDir, dbDir)
        tmpDir
      }.attempt.map(_.leftMap {
        case e: BlockstoreError => e
        case e                  => SymlinkCreationError(e, levelDbDir)
      })
    )(p => F.delay(p.foreach(rmDir)).attempt.void)
  }

  def createStore[F[_]: Log: Sync: LiftIO: ContextShift](
    dir: String
  ): Resource[F, Either[BlockstoreError, Blockstore[F]]] =
    createSymlinks[F](dir).flatMap(
      dbPath =>
        Log.resource[F].debug(s"Opening DB at $dbPath") *>
          Traverse[Either[BlockstoreError, ?]].sequence(
            dbPath.map(
              p =>
                RocksDBStore
                  .makeRaw[F](p.toString, createIfMissing = false, readOnly = true)
                  .map(kv => new Blockstore(kv))
            )
        )
    )
}
