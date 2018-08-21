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

package fluence.statemachine.tx

import scala.collection.mutable
import scala.util.Try
import scala.util.matching.Regex

/**
 * Invokes transactions. Stub, should be replaced by VM calls.
 */
class TxInvoker {
  private val BinaryOpPattern: Regex = "(.+)\\((.*),(.*)\\)".r
  private val UnaryOpPattern: Regex = "(.+)\\((.*)\\)".r
  private val PlainValuePattern: Regex = "(.*)".r

  /**
   * This is stub. Call to underlying VM is expected here.
   *
   * @param txPayload description of function invocation including function name and arguments
   * @return either successful invocation's result or failed invocation's error message
   */
  def invokeTx(txPayload: String): Either[String, String] = txPayload match {
    case BinaryOpPattern(op, arg1, arg2) =>
      op match {
        case "sum" => SumOperation(arg1, arg2)()
        case _ => Left("Unknown binary op")
      }
    case UnaryOpPattern(op, arg) =>
      op match {
        case "increment" => IncrementOperation(arg, internalState)()
        case "factorial" => FactorialOperation(arg)()
        case _ => Left("Unknown unary op")
      }
    case PlainValuePattern(value) => SetValueOperation(value)()
  }

  /**
   * This is stub for stateful operation support.
   */
  private val internalState = new OperationInternalState

}

/**
 * Trait for operation stubs. Everything below will be removed after integration with VM.
 */
trait Operation extends slogging.LazyLogging {
  def apply(): Either[String, String]

  protected def tryParseLong(txt: String): Either[String, Long] = Try(txt.toLong).toEither.left.map(_.getMessage)
}

class OperationInternalState {
  private val counters = mutable.HashMap[String, Long]().withDefaultValue(0)

  def incrementCounter(name: String): Long = {
    counters(name) += 1
    counters(name)
  }
}

case class SetValueOperation(value: String) extends Operation {
  override def apply(): Either[String, String] = {
    logger.trace("process set value={}", value)
    Right(value)
  }
}

case class IncrementOperation(arg: String, internalState: OperationInternalState) extends Operation {
  override def apply(): Either[String, String] = {
    logger.trace("process increment arg={}", arg)
    Right(internalState.incrementCounter(arg).toString)
  }
}

case class FactorialOperation(arg: String) extends Operation {
  override def apply(): Either[String, String] = {
    logger.trace("process factorial arg={}", arg)
    tryParseLong(arg)
      .filterOrElse(_ >= 0, "Argument cannot be negative")
      .map(value => (1L to value).product.toString)
  }
}

case class SumOperation(arg1: String, arg2: String) extends Operation {
  override def apply(): Either[String, String] = {
    logger.trace("process sum arg1={} arg2={}", arg1, arg2)
    for {
      arg1Value <- tryParseLong(arg1)
      arg2Value <- tryParseLong(arg2)
    } yield (arg1Value + arg2Value).toString
  }
}
