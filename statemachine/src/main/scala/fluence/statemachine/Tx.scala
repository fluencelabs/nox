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

package fluence.statemachine
import slogging.LazyLogging

import scala.util.Try

/**
 * Represents Tendermint transaction with ordering data
 *
 * @param head Ordering info
 * @param data Payload
 */
case class Tx(head: Tx.Head, data: Tx.Data)

object Tx extends LazyLogging {

  /**
   * Ordering info.
   *
   * @param session Session ID, unique nonempty string for each session
   * @param nonce Starting from 0, increment one by one, no gaps allowed
   */
  case class Head(session: String, nonce: Long) {
    override def toString: String = s"$session/$nonce"
  }

  /**
   * Payload
   */
  case class Data(value: Array[Byte]) extends AnyVal

  /** sessionId/234234 */
  private val pathR = "^([^/]+)/([0-9]+)$".r

  /**
   * Try to read [[Head]] from string input.
   *
   * @param path Usually Query path
   */
  def readHead(path: String): Option[Head] = path match {
    case pathR(session, nonce) ⇒ Try(Head(session, nonce.toLong)).toOption
    case _ ⇒ None
  }

  /**
   * Try to read binary transaction.
   *
   * @param tx Binary transaction, how it cames from Tendermint
   */
  def readTx(tx: Array[Byte]): Option[Tx] = {
    val headIndex = tx.indexWhere(_.toChar == '\n')
    if (headIndex <= 0) {
      None
    } else {
      val (head, tail) = tx.splitAt(headIndex)
      readHead(new String(head)).flatMap { h ⇒
        val t = Try(
          Tx(h, Data(tail.drop(1)))
        )
        t.failed.foreach(err ⇒ logger.error(s"Cannot parse tx: $err", err))
        t.toOption
      }
    }
  }
}
