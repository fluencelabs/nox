package fluence.btree.binary.kryo

import com.twitter.chill.KryoPool
import fluence.btree.binary.Codec

/**
 * Thread-safe implementation of [[Codec]] with ''Kryo'' serialization.
 */
class KryoCodec(kryoPool: KryoPool) extends Codec[Any, Array[Byte]] {

  override def encode(obj: Any): Array[Byte] = {
    kryoPool.toBytesWithClass(obj)
  }

  override def decode[T](binary: Array[Byte]): T = {
    (if (binary == null) null else kryoPool.fromBytes(binary)).asInstanceOf[T]
  }
}

object KryoCodec {

  def apply(kryoPool: KryoPool): KryoCodec = {
    new KryoCodec(kryoPool)
  }

  def apply(register: Seq[Class[_]] = Nil, registerRequired: Boolean = false): KryoCodec = {
    new KryoCodec(newKryoPool(register, registerRequired))
  }

  private def newKryoPool(
    registerClasses:  Seq[Class[_]],
    registerRequired: Boolean,
    poolSize:         Int           = Runtime.getRuntime.availableProcessors
  ): KryoPool = {
    KryoPool.withByteArrayOutputStream(poolSize, KryoFactory(registerClasses, registerRequired))
  }

}
