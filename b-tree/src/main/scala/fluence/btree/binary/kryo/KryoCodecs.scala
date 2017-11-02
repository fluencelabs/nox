package fluence.btree.binary.kryo

import cats.ApplicativeError
import cats.syntax.applicative._
import com.twitter.chill.KryoPool
import fluence.btree.binary.Codec
import shapeless._

import scala.language.higherKinds
import scala.reflect.ClassTag

/**
 * Wrapper for a KryoPool with a list of registered classes
 * @param pool Pre-configured KryoPool
 * @param F Applicative error
 * @tparam L List of classes registered with kryo
 * @tparam F Effect
 */
class KryoCodecs[L <: HList, F[_]] private (pool: KryoPool)(implicit F: ApplicativeError[F, Throwable]) {

  /**
   * Returns a codec for any registered type
   * @param sel Shows the presence of type T within list L
   * @tparam T Object type
   * @return Freshly created Codec with Kryo inside
   */
  implicit def codec[T](implicit sel: ops.hlist.Selector[L, T]): Codec[T, Array[Byte], F] =
    new Codec[T, Array[Byte], F] {
      override def encode(obj: T): F[Array[Byte]] =
        Option(obj) match {
          case Some(o) ⇒
            pool.toBytesWithClass(o).pure[F]
          case None ⇒
            F.raiseError[Array[Byte]](new NullPointerException("Obj is null, encoding is impossible"))
        }

      override def decode(binary: Array[Byte]): F[T] =
        F.catchNonFatal(pool.fromBytes(binary).asInstanceOf[T])
    }
}

object KryoCodecs {

  /**
   * Builder for Kryo codecs
   * @param klasses Classes to register with Kryo
   * @tparam L List of registered classes
   */
  class Builder[L <: HList] private[KryoCodecs] (klasses: Seq[Class[_]]) {
    /**
     * Register a new case class T to Kryo
     * @tparam T Type to add
     * @tparam S Generic representation of T
     * @param gen Generic representation of case type T
     * @param sa Presence of all types of S inside L
     * @return Extended builder
     */
    def addCase[T, S <: HList](klass: Class[T])(implicit gen: Generic.Aux[T, S], sa: ops.hlist.SelectAll[L, S]): Builder[T :: L] =
      new Builder[T :: L](klasses :+ klass)

    /**
     * Register a primitive type T to Kryo
     * @tparam T Type to add
     * @return Extended builder
     */
    def add[T : ClassTag]: Builder[T :: L] =
      new Builder[T :: L](klasses :+ implicitly[ClassTag[T]].runtimeClass)

    /**
     * Build a new instance of KryoCodecs with the given poolSize and F effect
     * @param poolSize Kryo pool size
     * @param F ApplicativeError for catching serialization errors
     * @tparam F Effect type
     * @return Configured instance of KryoCodecs
     */
    def build[F[_]](poolSize: Int = Runtime.getRuntime.availableProcessors)(implicit F: ApplicativeError[F, Throwable]): KryoCodecs[L, F] =
      new KryoCodecs[L, F](
        KryoPool.withByteArrayOutputStream(
          poolSize,
          KryoFactory(klasses, registrationRequired = true) // registrationRequired should never be needed, as codec derivation is typesafe
        )
      )
  }

  /**
   * Prepares a fresh builder
   */
  def apply(): Builder[Array[Byte] :: Long :: String :: HNil] =
    new Builder[HNil](Vector.empty).add[String].add[Long].add[Array[Byte]]
}
