/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.storage.rocksdb

import java.nio.ByteBuffer

import cats.instances.try_._
import cats.~>
import com.typesafe.config.ConfigFactory
import fluence.codec.Codec
import fluence.storage.rocksdb.RocksDbStore.{ Key, Value }
import monix.eval.Task
import monix.execution.{ ExecutionModel, Scheduler }
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.time.{ Milliseconds, Seconds, Span }
import org.scalatest.{ BeforeAndAfterAll, Matchers, WordSpec }

import scala.language.{ higherKinds, implicitConversions }
import scala.reflect.io.Path
import scala.util.{ Random, Try }

class IdSeqProviderSpec extends WordSpec with Matchers with ScalaFutures with BeforeAndAfterAll {
  override implicit def patienceConfig: PatienceConfig = PatienceConfig(Span(1, Seconds), Span(250, Milliseconds))
  implicit val scheduler: Scheduler = Scheduler(ExecutionModel.AlwaysAsyncExecution)

  private val conf = RocksDbConf.read[Try](ConfigFactory.load()).get
  assert(conf.dataDir.startsWith(System.getProperty("java.io.tmpdir")))

  private implicit val valRef2bytesCodec: Codec[Task, Array[Byte], Long] = Codec.pure(
    ByteBuffer.wrap(_).getLong(),
    ByteBuffer.allocate(java.lang.Long.BYTES).putLong(_).array()
  )

  private implicit def long2Bytes(long: Long): Array[Byte] = {
    ByteBuffer.allocate(java.lang.Long.BYTES).putLong(long).array()
  }

  private implicit def runId[F[_]]: F ~> F = new (F ~> F) {
    override def apply[A](fa: F[A]): F[A] = fa
  }

  "longSeqProvider" should {

    "create in memory long id provider from local rocksDb" when {

      "rocksDb is empty" in {
        runRocksDb("IdSeqProviderSpec.test3") { rocksDb ⇒
          val idProvider = IdSeqProvider.longSeqProvider[Task](rocksDb, 10L).runAsync.futureValue
          idProvider() shouldBe 11L
        }
      }

      "rocksDb is filled" in {
        runRocksDb("IdSeqProviderSpec.test3") { rocksDb ⇒
          val manyPairs: Seq[(Key, Value)] = Random.shuffle(1 to 100).map { n ⇒ long2Bytes(n) → s"val$n".getBytes() }
          val inserts = manyPairs.map { case (k, v) ⇒ rocksDb.put(k, v) }
          Task.sequence(inserts).flatMap(_ ⇒ rocksDb.traverse().toListL).runAsync.futureValue

          val idProvider = IdSeqProvider.longSeqProvider[Task](rocksDb, 10L).runAsync.futureValue
          idProvider() shouldBe 101L
        }
      }
    }
  }

  private def runRocksDb(name: String)(action: RocksDbStore ⇒ Unit): Unit = {
    val store = new RocksDbStore.Factory()(name, conf).get
    try action(store) finally store.close()
  }

  override protected def afterAll(): Unit = {
    Path(conf.dataDir).deleteRecursively()
  }

}
