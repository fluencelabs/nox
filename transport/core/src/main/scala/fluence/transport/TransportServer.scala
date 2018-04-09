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

package fluence.transport

import java.util.concurrent.atomic.AtomicReference

import cats.effect.IO

/**
 * Trait to control server lifecycle.
 *
 * @tparam B Server builder.
 * @tparam S Server implementation.
 */
trait TransportServer[B, S] {

  protected val serverRef = new AtomicReference[Option[S]](None)

  protected def onStart: IO[Unit]
  protected def onShutdown: IO[Unit]

  protected def builder: IO[B]

  protected def startServer: B ⇒ IO[S]
  protected def shutdownServer: S ⇒ IO[Unit]

  /**
   * Launch server, grab ports, or fail
   */
  val start: IO[Unit] =
    for {
      _ ← if (serverRef.get() == null) IO.unit else shutdown
      ser ← builder
      s ← startServer(ser)
      _ ← onStart
    } yield {
      serverRef.set(Some(s))
    }

  /**
   * Shut the server down, release ports
   */
  lazy val shutdown: IO[Unit] =
    serverRef
      .getAndSet(None)
      .fold(IO.unit)(
        srv ⇒
          for {
            _ ← onShutdown
            _ ← shutdownServer(srv)
          } yield {}
      )
}
