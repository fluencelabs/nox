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

package fluence.statemachine.control

import cats.Order
import cats.syntax.flatMap._

import scala.language.higherKinds

/**
 * Describes any data that can expose a property with order defined on it
 *
 * @tparam A Type of the data
 * @tparam T Type of the property being exposed
 */
trait HasOrderedProperty[A, T] {
  def key(a: A)(implicit o: Order[T]): T
}

object HasOrderedProperty {
  implicit val vmHash: HasOrderedProperty[VmHash, Long] = new HasOrderedProperty[VmHash, Long] {
    override def key(a: VmHash)(implicit o: Order[Long]): Long = a.height
  }
  implicit val receipt: HasOrderedProperty[BlockReceipt, Long] = new HasOrderedProperty[BlockReceipt, Long] {
    override def key(a: BlockReceipt)(implicit o: Order[Long]): Long = a.receipt.height
  }

  def apply[A: HasOrderedProperty[*, T], T: Order]: HasOrderedProperty[A, T] =
    implicitly[HasOrderedProperty[A, T]]

  object syntax {
    implicit class HasOrderedPropertySyntax[A: HasOrderedProperty[*, T], T: Order](a: A) {
      def key: T = HasOrderedProperty[A, T].key(a)
    }
  }
}
