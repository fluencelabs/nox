package fluence.btree.binary.kryo

import com.twitter.chill.KryoPool
import fluence.btree.binary.Codec

import scala.util.Try

/**
 * Thread-safe implementation of [[Codec]] with ''Kryo'' serialization.
 */
class KryoCodec[T](kryoPool: KryoPool) extends Codec[T, Array[Byte], Option] {

  override def encode(obj: T): Option[Array[Byte]] = {
    Option(obj).map(kryoPool.toBytesWithClass)
  }

  override def decode(binary: Array[Byte]): Option[T] = {
    Try(kryoPool.fromBytes(binary).asInstanceOf[T]).toOption
  }
}

object KryoCodec {

  def apply[T](kryoPool: KryoPool): KryoCodec[T] = {
    new KryoCodec[T](kryoPool)
  }

  /**
   * This Instantiator enable compulsory class registration, registers all java and scala main classes.
   * This class required for [[com.twitter.chill.KryoPool]].
   * @param classesToReg additional classes for registration
   * @param registerRequired if true, an exception is thrown when an unregistered class is encountered.
   */
  def apply[T](classesToReg: Seq[Class[_]] = Nil, registerRequired: Boolean = false): KryoCodec[T] = {
    new KryoCodec[T](newKryoPool(classesToReg, registerRequired))
  }

  private def newKryoPool(
    registerClasses:  Seq[Class[_]],
    registerRequired: Boolean,
    poolSize:         Int           = Runtime.getRuntime.availableProcessors
  ): KryoPool = {
    KryoPool.withByteArrayOutputStream(poolSize, KryoFactory(registerClasses, registerRequired))
  }

}
