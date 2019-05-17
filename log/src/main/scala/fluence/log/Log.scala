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

import cats.data.Chain
import cats.{Eval, Order}
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.effect.{Clock, Sync}
import cats.effect.concurrent.Ref

import scala.language.higherKinds

/**
 * Functional logger facade
 *
 * @param ctx Trace Context
 * @param data Log Data
 * @tparam F Effect
 */
class Log[F[_]: Sync: Clock](val ctx: Context, private val data: Ref[F, Chain[Log.Msg]]) {

  private val millis = Clock[F].realTime(TimeUnit.MILLISECONDS)

  /**
   * Provide a logger with modified context
   *
   * @param modContext Context modification
   * @param fn Function to use the new logger
   * @tparam A Return type
   * @return What the inner function returns
   */
  def scope[A](modContext: Context ⇒ Context)(fn: Log[F] ⇒ F[A]): F[A] =
    fn(new Log(modContext(ctx), data))

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
    append(Log.Trace, Eval.later(msg), None)

  def debug(msg: ⇒ String): F[Unit] =
    append(Log.Debug, Eval.later(msg), None)

  def info(msg: ⇒ String): F[Unit] =
    append(Log.Info, Eval.later(msg), None)

  def warn(msg: ⇒ String, cause: Throwable = null): F[Unit] =
    append(Log.Warn, Eval.later(msg), Option(cause))

  def error(msg: ⇒ String, cause: Throwable = null): F[Unit] =
    append(Log.Error, Eval.later(msg), Option(cause))

  private def append(level: Log.Level, msg: Eval[String], cause: Option[Throwable]): F[Unit] =
    millis >>= (m ⇒ data.update(_.append(Log.Msg(m, level, ctx, msg, cause))))

  def mkString(level: Log.Level = Log.Trace): F[String] =
    data.get.map(_.iterator.filter(_.level.flag >= level.flag).mkString("\n"))
}

object Log {

  /**
   * Summoner
   */
  def apply[F[_]](implicit log: Log[F]): Log[F] = log

  private val dateFormat = new SimpleDateFormat("yyyy-MM-dd HH:mm:ss")

  case class Msg(timestamp: Long, level: Level, ctx: Context, msg: Eval[String], cause: Option[Throwable]) {
    def date = dateFormat.format(new Date(timestamp))

    override def toString: String =
      s"${Console.WHITE}$date${Console.RESET} ${level.color}${level.name}${Console.RESET} $ctx\t${msg.value}" + cause
        .fold("")(c ⇒ s"\tcaused by: $c")
  }

  sealed abstract class Level(val flag: Int, val name: String, val color: String)
  case object Trace extends Level(0, "trace", Console.WHITE)
  case object Debug extends Level(1, "debug", Console.MAGENTA)
  case object Info extends Level(2, "info ", Console.BLUE)
  case object Warn extends Level(3, "warn ", Console.RED)
  case object Error extends Level(4, "error", Console.RED + Console.BOLD)

  implicit val LevelOrder: Order[Level] =
    Order.by[Level, Int](_.flag)(Order.fromOrdering[Int])

  implicit def forCtx[F[_]: Sync: Clock](implicit ctx: Context): Log[F] =
    new Log[F](ctx, Ref.unsafe(Chain.empty))
}
