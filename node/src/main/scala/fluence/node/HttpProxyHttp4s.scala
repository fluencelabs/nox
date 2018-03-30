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

package fluence.node

import cats.data.EitherT
import cats.effect._
import org.http4s._
import org.http4s.dsl.io._
import org.http4s.server.blaze.BlazeBuilder

import scala.concurrent.ExecutionContext.Implicits.global

object HttpProxyHttp4s extends slogging.LazyLogging {

  val helloWorldService = HttpService[IO] {
    case req @ GET ⇒
      logger.error(req.toString())
      logger.error(s"Hello, get.")
      Ok(s"Hello, get.")
    case req @ POST -> path ⇒
      logger.error(req.toString())
      logger.error(s"Hello, post.")
      val stringReq = req.as[Array[Byte]]
      logger.error(s"Req as string === " + stringReq.unsafeRunSync().mkString(","))
      Ok(s"Hello, post.")
    case req ⇒
      logger.error(req.toString())
      logger.error("hi rec")
      Ok("Hi req")
  }

  val builder = BlazeBuilder[IO].bindHttp(8080).mountService(helloWorldService, "/").start
}
