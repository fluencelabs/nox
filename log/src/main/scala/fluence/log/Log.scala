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

package fluence.log

import java.text.SimpleDateFormat
import java.util.Date
import java.util.concurrent.TimeUnit

import cats.{Applicative, Eval, Order}
import cats.effect.{Clock, Sync}
import cats.syntax.order._
import cats.syntax.flatMap._

import scala.language.higherKinds

/**
 * Functional logger facade
 *
 * @tparam F Effect
 */
abstract class Log[F[_]: Sync: Clock] {

  private val unit = Applicative[F].unit

  val ctx: Context

  import ctx.loggingLevel

  private val millis: F[Long] = Clock[F].realTime(TimeUnit.MILLISECONDS)

  /**
   * Provide a logger with modified context
   *
   * @param modContext Context modification
   * @param fn Function to use the new logger
   * @tparam A Return type
   * @return What the inner function returns
   */
  def scope[A](modContext: Context ⇒ Context)(fn: Log[F] ⇒ F[A]): F[A]

  /**
   * Provide a logger with modified context
   *
   * @param kvs Key-value pairs to modify the context
   * @param fn Function to use the new logger
   * @tparam A Return type
   * @return What the inner function returns
   */
  def scope[A](kvs: (String, String)*)(fn: Log[F] ⇒ F[A]): F[A] =
    scope(_.scope(kvs: _*))(fn)

  /**
   * Provide a logger with modified context
   *
   * @param k Key to modify the context (value will be empty)
   * @param fn Function to use the new logger
   * @tparam A Return type
   * @return What the inner function returns
   */
  def scope[A](k: String)(fn: Log[F] ⇒ F[A]): F[A] =
    scope(_.scope(k -> ""))(fn)

  def trace(msg: ⇒ String): F[Unit] =
    if (loggingLevel <= Log.Trace) append(Log.Trace, Eval.later(msg), None) else unit

  def debug(msg: ⇒ String): F[Unit] =
    if (loggingLevel <= Log.Debug) append(Log.Debug, Eval.later(msg), None) else unit

  def info(msg: ⇒ String): F[Unit] =
    if (loggingLevel <= Log.Info) append(Log.Info, Eval.later(msg), None) else unit

  def warn(msg: ⇒ String, cause: Throwable = null): F[Unit] =
    if (loggingLevel <= Log.Warn) append(Log.Warn, Eval.later(msg), Option(cause)) else unit

  def error(msg: ⇒ String, cause: Throwable = null): F[Unit] =
    if (loggingLevel <= Log.Error) append(Log.Error, Eval.later(msg), Option(cause)) else unit

  private def append(level: Log.Level, msg: Eval[String], cause: Option[Throwable]): F[Unit] =
    millis >>= (m ⇒ appendMsg(Log.Msg(m, level, ctx, msg, cause)))

  protected def appendMsg(msg: Log.Msg): F[Unit]
}

object Log {

  /**
   * Summoner
   */
  def apply[F[_]](implicit log: Log[F]): Log[F] = log

  private val dateFormat = new SimpleDateFormat("yyyy-MM-dd HH:mm:ss")

  case class Msg(timestamp: Long, level: Level, ctx: Context, msg: Eval[String], cause: Option[Throwable]) {
    private def date = dateFormat.format(new Date(timestamp))

    override def toString: String =
      s"${Console.WHITE}$date${Console.RESET} ${level.color}${level.name}${Console.RESET} $ctx\t${msg.value}" +
        cause.fold("")(c ⇒ s"\tcaused by: $c")
  }

  sealed abstract class Level(val flag: Int, val name: String, val color: String)
  case object Trace extends Level(0, "trace", Console.WHITE)
  case object Debug extends Level(1, "debug", Console.MAGENTA)
  case object Info extends Level(2, "info ", Console.BLUE)
  case object Warn extends Level(3, "warn ", Console.RED)
  case object Error extends Level(4, "error", Console.RED + Console.BOLD)

  implicit val LevelOrder: Order[Level] =
    Order.by[Level, Int](_.flag)(Order.fromOrdering[Int])

}
