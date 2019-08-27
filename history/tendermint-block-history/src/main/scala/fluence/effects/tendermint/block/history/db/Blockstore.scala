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
import java.util.UUID

import cats.{Functor, Monad, Traverse}
import cats.data.EitherT
import cats.effect.{ContextShift, ExitCode, IO, IOApp, LiftIO, Resource, Sync}
import cats.instances.either._
import cats.instances.list._
import cats.syntax.applicativeError._
import cats.syntax.apply._
import cats.syntax.functor._
import cats.syntax.either._
import cats.syntax.traverse._
import cats.syntax.flatMap._
import cats.syntax.foldable._
import fluence.effects.kvstore.{KVStore, RocksDBStore}
import fluence.effects.tendermint.block.TendermintBlock
import fluence.effects.tendermint.block.protobuf.{Protobuf, ProtobufConverter}
import fluence.effects.{EffectError, WithCause}
import fluence.log.Log.Aux
import fluence.log.appender.PrintlnLogAppender
import fluence.log.{Log, LogFactory}
import proto3.tendermint.{Block, BlockMeta, BlockPart}

import scala.collection.JavaConverters._
import scala.language.higherKinds
import scala.util.Try

// Here's how different data is stored in blockstore.db
/*
func calcBlockMetaKey(height int64) []byte {
	return []byte(fmt.Sprintf("H:%v", height))
}

func calcBlockPartKey(height int64, partIndex int) []byte {
	return []byte(fmt.Sprintf("P:%v:%v", height, partIndex))
}

func calcBlockCommitKey(height int64) []byte {
	return []byte(fmt.Sprintf("C:%v", height))
}

func calcSeenCommitKey(height int64) []byte {
	return []byte(fmt.Sprintf("SC:%v", height))
}
 */

trait BlockstoreError extends EffectError
case class DbNotFound(path: String) extends BlockstoreError {
  override def toString: String = s"DbNotFound: can't find leveldb database at path $path: path doesn't exist"
}
case object NoTmpDirPropertyError extends BlockstoreError {
  override def toString: String = "NoTmpDirPropertyError: java.io.tmpdir is empty"
}
case class GetBlockError(message: String, height: Long) extends BlockstoreError {
  override def toString: String = s"GetBlockError: $message on block $height"
}
case class SymlinkCreationError(cause: Throwable, target: String) extends BlockstoreError with WithCause[Throwable] {
  override def toString: String = s"SymlinkCreationError: error creating symlink for $target: $cause"
}
case class RetrievingStorageHeightError(cause: Throwable) extends EffectError {
  override def toString: String = s"RetrievingStorageHeightError: $cause"
}

/**
 * Read blocks from blockstore.db which is at
 * `~.fluence/app-89-3/tendermint/data/blockstore.db`
 */
object Blockstore extends IOApp {
  override def run(args: List[String]): IO[ExitCode] =
    LogFactory
      .forPrintln[IO](Log.Trace)
      .init()
      .flatMap { implicit log =>
        val name =
          "/Users/folex/Development/fluencelabs/fluence-main/history/tendermint-block-history/src/main/scala/fluence/effects/tendermint/block/history/db/blockstore.db"
        createKV[IO](name).use { e =>
          Traverse[Either[BlockstoreError, *]].sequence(
            e.map { kvStore =>
              getStorageHeight(kvStore)
                .flatMap(h => (1 to h).toList.traverse(getBlock(kvStore, _)))
                .flatTap(
                  _.traverse(
                    v =>
                      Log
                        .eitherT[IO, Throwable]
                        .info(
//                            s"${v} ${v.data}\n\n" +
//                            s"${ProtobufConverter.fromProtobuf(v)}\n\n" +
                          s"${v.header.get.height} => ${ProtobufConverter.fromProtobuf(v).flatMap(TendermintBlock(_).validateHashes()).fold(e => s"FAIL: $e", _ => "OK")}"
                      )
                  )
                )
                .value
            }
          )
        }
      }
      .map(_ => ExitCode.Success)

  def metaKey(height: Long) = s"H:$height".getBytes()
  def partKey(height: Long, index: Int) = s"P:$height:$index".getBytes()

  private def getOr[T, F[_]: Monad](msg: String, height: Long)(opt: Option[T]): EitherT[F, Throwable, T] =
    EitherT.fromOption(opt, GetBlockError(msg, height))

  def createKV[F[_]: Log: Sync: LiftIO: ContextShift](
    dir: String
  ): Resource[F, Either[BlockstoreError, KVStore[F, Array[Byte], Array[Byte]]]] =
    createSymlinks[F](dir).flatMap(
      dbPath =>
        Log.resource[F].debug(s"Opening DB at $dbPath") *>
          Traverse[Either[BlockstoreError, *]].sequence(
            dbPath.map(p => RocksDBStore.makeRaw[F](p.toString, createIfMissing = false, readOnly = true))
        )
    )

  def createSymlinks[F[_]: Monad](levelDbDir: String)(implicit F: Sync[F]): Resource[F, Either[BlockstoreError, Path]] =
    Resource.make(F.delay {
      val tmpDir = Files.createTempDirectory("leveldb_rocksdb")
      val dbDir = Paths.get(levelDbDir).toAbsolutePath
      Files.list(dbDir).iterator().asScala.foreach { file =>
        val fname = {
          val n = file.getFileName.toString
          if (n.endsWith(".ldb")) n.replaceFirst(".ldb$", ".sst")
          else n
        }
        val link = tmpDir.resolve(fname)
        Files.createSymbolicLink(link, file)
      }
      tmpDir
    }.attempt.map(_.leftMap {
      case e: BlockstoreError => e
      case e                  => SymlinkCreationError(e, levelDbDir)
    }))(
      p =>
        F.delay {
          p.foreach { dir =>
            Files.list(dir).iterator().asScala.foreach { file =>
              Files.delete(file)
            }
            Files.delete(dir)
          }
        }.attempt.void
    )

  def getBlock[F[_]: Log: Monad](kv: KVStore[F, Array[Byte], Array[Byte]], height: Long): EitherT[F, Throwable, Block] =
    for {
      metaBytes <- kv
        .get(metaKey(height))
        .flatMap(getOr[Array[Byte], F]("meta is none", height)(_))
      meta <- EitherT.fromEither[F](Protobuf.decode[BlockMeta](metaBytes))

      partsCount <- getOr[Int, F]("blockID.parts is none", height)(meta.blockID.flatMap(_.parts).map(_.total))

      getPart = (i: Int) =>
        kv.get(partKey(height, i))
          .flatMap(getOr[Array[Byte], F](s"part $i not found", height))
          .subflatMap(Protobuf.decode[BlockPart])

      blockBytes <- (0 until partsCount).toList.foldM(Array.empty[Byte]) {
        case (bytes, idx) => getPart(idx).map(bytes ++ _.bytes.toByteArray)
      }
      block <- EitherT.fromEither[F](Protobuf.decodeLengthPrefixed[Block](blockBytes))
    } yield block

  val BlockStoreHeightKey = "blockStore".getBytes

  import io.circe.parser.parse

  def getStorageHeight[F[_]: Monad: Log](kv: KVStore[F, Array[Byte], Array[Byte]]): EitherT[F, EffectError, Int] =
    (for {
      heightJsonBytes <- kv.get(BlockStoreHeightKey).flatMap(getOr("blockStore height wasn't found", -1)(_))
      height <- EitherT.fromEither[F](
        Try(new String(heightJsonBytes)).toEither >>= parse >>= (_.hcursor.get[Int]("height"))
      )
    } yield height).leftMap(RetrievingStorageHeightError)
}
