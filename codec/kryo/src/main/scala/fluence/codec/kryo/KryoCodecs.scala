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

package fluence.codec.kryo

import cats.MonadError
import cats.syntax.flatMap._
import com.twitter.chill.KryoPool
import fluence.codec.Codec
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
class KryoCodecs[F[_], L <: HList] private (pool: KryoPool)(implicit F: MonadError[F, Throwable]) {

  /**
   * Returns a codec for any registered type
   * @param sel Shows the presence of type T within list L
   * @tparam T Object type
   * @return Freshly created Codec with Kryo inside
   */
  implicit def codec[T](implicit sel: ops.hlist.Selector[L, T]): Codec[F, T, Array[Byte]] =
    Codec(
      obj ⇒
        Option(obj) match {
          case Some(o) ⇒
            F.catchNonFatal(Option(pool.toBytesWithClass(o))).flatMap {
              case Some(v) ⇒ F.pure(v)
              case None ⇒ F.raiseError(new NullPointerException("Obj is encoded into null"))
            }
          case None ⇒
            F.raiseError[Array[Byte]](new NullPointerException("Obj is null, encoding is impossible"))
      },
      binary ⇒ F.catchNonFatal(pool.fromBytes(binary).asInstanceOf[T])
    )
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
    def addCase[T, S <: HList](
      klass: Class[T])(implicit gen: Generic.Aux[T, S], sa: ops.hlist.SelectAll[L, S]): Builder[T :: L] =
      new Builder[T :: L](klasses :+ klass)

    /**
     * Register a primitive type T to Kryo
     * @tparam T Type to add
     * @return Extended builder
     */
    def add[T: ClassTag]: Builder[T :: L] =
      new Builder[T :: L](klasses :+ implicitly[ClassTag[T]].runtimeClass)

    /**
     * Build a new instance of KryoCodecs with the given poolSize and F effect
     * @param poolSize Kryo pool size
     * @param F ApplicativeError for catching serialization errors
     * @tparam F Effect type
     * @return Configured instance of KryoCodecs
     */
    def build[F[_]](poolSize: Int = Runtime.getRuntime.availableProcessors)(
      implicit F: MonadError[F, Throwable]): KryoCodecs[F, L] =
      new KryoCodecs[F, L](
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
