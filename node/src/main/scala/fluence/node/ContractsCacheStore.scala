package fluence.node

import java.nio.ByteBuffer
import java.time.Instant

import fluence.codec.Codec
import fluence.dataset.BasicContract
import fluence.dataset.grpc.BasicContractCodec
import fluence.dataset.node.contract.ContractRecord
import fluence.kad.protocol.Key
import fluence.storage.KVStore
import fluence.storage.rocksdb.RocksDbStore
import monix.eval.Task

import scala.language.higherKinds

object ContractsCacheStore {

  def apply(): KVStore[Task, Key, ContractRecord[BasicContract]] = {
    import Key.bytesCodec

    implicit val contractRecordCodec: Codec[Task, ContractRecord[BasicContract], Array[Byte]] = {
      val grpcCodec = BasicContractCodec.codec[Task]
      Codec[Task, ContractRecord[BasicContract], Array[Byte]](
        cr ⇒ grpcCodec.encode(cr.contract).map(_.toByteArray).map(Array.concat(
          ByteBuffer.allocate(java.lang.Long.BYTES).putLong(cr.lastUpdated.toEpochMilli).array(), _)),

        bs ⇒
          for {
            timec ← Task(bs.splitAt(java.lang.Long.BYTES))
            ts = Instant.ofEpochMilli(ByteBuffer.wrap(timec._1).getLong)
            bcg = fluence.dataset.grpc.BasicContract.parseFrom(timec._2)
            bc ← grpcCodec.decode(bcg)
          } yield ContractRecord(bc, ts)
      )
    }

    RocksDbStore("fluence:contractsCache").get
  }
}
