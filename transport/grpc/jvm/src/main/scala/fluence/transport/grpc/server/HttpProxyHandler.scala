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

package fluence.transport.grpc.server

import com.google.logging.`type`.HttpRequest
import io.netty.buffer.Unpooled
import io.netty.channel.{ChannelFutureListener, ChannelHandlerContext, SimpleChannelInboundHandler}
import io.netty.handler.codec.http.{DefaultFullHttpResponse, HttpContent}
import io.netty.handler.codec.http.HttpResponseStatus._
import io.netty.handler.codec.http.HttpVersion._

import scala.util.Try

class HttpProxyHandler extends SimpleChannelInboundHandler[Object] with slogging.LazyLogging {

  override def channelRead0(ctx: ChannelHandlerContext, msg: Object): Unit = {
    Try {
      msg match {
        case hr: HttpRequest ⇒ logger.error("HTTPREQUEST === " + hr)
        case hc: HttpContent ⇒ logger.error("CONTENT === " + hc)
        case req ⇒ logger.error("REQUEST === " + req)
      }

      val resp = new DefaultFullHttpResponse(HTTP_1_1, OK)
      ctx.write(resp)
      ctx.writeAndFlush(Unpooled.EMPTY_BUFFER).addListener(ChannelFutureListener.CLOSE)
    }.recover {
      case e: Throwable ⇒ logger.error("Some error ", e)
    }
  }
}
