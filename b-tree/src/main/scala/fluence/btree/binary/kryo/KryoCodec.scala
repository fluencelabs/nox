package fluence.btree.binary.kryo

import com.twitter.chill.KryoPool
import fluence.btree.binary.Codec

/**
 * Thread-safe implementation of [[Codec]] with ''Kryo'' serialization.
 */
class KryoCodec(kryoPool: KryoPool) extends Codec[Any, Array[Byte]] {

  override def encode(obj: Any): Array[Byte] = {
    assert(kryoPool.hasRegistration(obj.getClass), s"${obj.getClass} should be registered in Kryo")
    kryoPool.toBytesWithClass(obj)
  }

  override def decode(binary: Array[Byte]): AnyRef = {
    kryoPool.fromBytes(binary)
  }
}

object KryoCodec {

  def apply(kryoPool: KryoPool): KryoCodec = {
    new KryoCodec(kryoPool)
  }

  def apply(register: Seq[Class[_]]): KryoCodec = {
    this(newKryoPool(register))
  }

  private def newKryoPool(register: Seq[Class[_]], poolSize: Int = Runtime.getRuntime.availableProcessors): KryoPool = {
    KryoPool.withByteArrayOutputStream(poolSize, StrictKryoInstantiator(register))
  }

}
