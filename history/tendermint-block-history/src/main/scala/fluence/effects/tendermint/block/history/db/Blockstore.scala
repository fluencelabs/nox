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

import cats.effect.{ExitCode, IO, IOApp}
import fluence.effects.kvstore.RocksDBStore
import fluence.log.{Log, LogFactory}
import cats.syntax.apply._

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
              case (k, v) => log.info(s"k: $k -> v: $v}")
            }.compile.toList
              .flatMap(l => log.info(s"list: $l"))

        }
      }
      .map(_ => ExitCode.Success)
  }
}
