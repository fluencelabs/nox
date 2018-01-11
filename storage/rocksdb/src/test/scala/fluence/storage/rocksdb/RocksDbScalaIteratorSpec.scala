package fluence.storage.rocksdb

import org.mockito.Mockito
import org.mockito.Mockito._
import org.rocksdb.RocksIterator
import org.scalatest.{ Matchers, WordSpec }

class RocksDbScalaIteratorSpec extends WordSpec with Matchers {

  "RocksDbScalaIterator" should {

    "wrap RocksIterator correct" in {

      val Key1 = "key1".getBytes
      val Key2 = "key2".getBytes
      val Value1 = "value1".getBytes
      val Value2 = "value2".getBytes

      val rocksIterator = mock(classOf[RocksIterator])
      when(rocksIterator.isValid).thenReturn(true, true, false)
      when(rocksIterator.key).thenReturn(Key1, Key2)
      when(rocksIterator.value).thenReturn(Value1, Value2)

      val iterator = RocksDbScalaIterator(rocksIterator)

      val result = iterator.toList

      result should contain inOrder (
        Key1 → Value1,
        Key2 → Value2
      )
      verify(rocksIterator, Mockito.times(1)).seekToFirst()
      verify(rocksIterator, Mockito.times(2)).next()
      verify(rocksIterator, Mockito.times(3)).isValid
      verify(rocksIterator, Mockito.times(1)).close()
    }
  }

}
