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

import cats.{Foldable, Functor, Monad}
import cats.data.EitherT
import cats.effect.{ExitCode, IO, IOApp}
import fluence.effects.kvstore.{KVStore, RocksDBStore}
import fluence.log.{Log, LogFactory}
import cats.syntax.apply._
import fluence.effects.tendermint.block.protobuf.Protobuf
import proto3.tendermint.{Block, BlockMeta, BlockPart}
import scodec.bits.ByteVector

import scala.language.higherKinds
import scala.util.control.NoStackTrace

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

/**
 * Read blocks from blockstore.db which is at
 * `~.fluence/app-89-3/tendermint/data/blockstore.db`
 */
object Blockstore extends IOApp {
  override def run(args: List[String]): IO[ExitCode] = {

    LogFactory
      .forPrintln[IO](Log.Trace)
      .init()
      .flatMap { implicit log =>
        val name =
          "/Users/folex/Development/fluencelabs/fluence-main/history/tendermint-block-history/src/main/scala/fluence/effects/tendermint/block/history/db/blockstore_rcksdb.db"
        RocksDBStore.makeRaw[IO](name, createIfMissing = false).use { kvStore =>
          log.info("Heeey!") *>
            kvStore.stream.evalMap {
              case (k, v) => IO((new String(k), new String(v)))
            }.evalTap {
              case (k, v) => IO.unit //log.info(s"k: $k")
            }.compile.toList *>
            getBlock(kvStore, 500).value.flatMap(v => log.info(s"value: ${v}")) *>
            kvStore.get(partKey(500, 0)).value.flatMap(v => log.info(s"part: ${v}"))
        }
      }
      .map(_ => ExitCode.Success)
  }

  def metaKey(height: Long) = s"H:$height".getBytes()
  def partKey(height: Long, index: Int) = s"P:$height:$index".getBytes()

  case class GetBlockError(message: String, height: Long) extends NoStackTrace {
    override def toString: String = s"GetBlockError: $message on block $height"
  }

  def getOr[T, F[_]: Monad](msg: String, height: Long)(opt: Option[T]): EitherT[F, Throwable, T] =
    EitherT.fromOption(opt, GetBlockError(msg, height))

  import cats.instances.list._
  import cats.syntax.foldable._

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
}
