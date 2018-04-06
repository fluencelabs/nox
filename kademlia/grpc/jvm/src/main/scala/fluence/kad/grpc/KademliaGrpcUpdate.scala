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

package fluence.kad.grpc

import java.time.Instant

import cats.Id
import cats.data.Kleisli
import cats.effect.IO
import fluence.crypto.SignAlgo.CheckerFn
import fluence.kad.protocol
import fluence.kad.protocol.{Contact, Key}
import fluence.transport.grpc.GrpcConf

object KademliaGrpcUpdate {

  /**
   * Provides a GRPC server callback to update Kademlia routing table
   *
   * @param update Kademlia.update function, converted to IO properly
   * @param clientConf Grpc Client conf, to get header names from
   * @return Callback to be registered on GRPC transport
   */
  def grpcCallback(
    update: fluence.kad.protocol.Node[Contact] ⇒ IO[Unit],
    clientConf: GrpcConf
  )(
    implicit checkerFn: CheckerFn
  ): (Kleisli[Option, String, String], Option[Any]) ⇒ IO[Unit] = { (headers, message) ⇒
    val remoteNode = for {
      remoteB64key ← headers(clientConf.keyHeader)
      remoteKey ← Key.fromB64[Id](remoteB64key).value.toOption

      remoteSeed ← headers(clientConf.contactHeader)
      remoteContact ← Contact.readB64seed[Id](remoteSeed).value.toOption
    } yield protocol.Node(remoteKey, Instant.now(), remoteContact)

    remoteNode.fold(IO.unit)(update)
  }
}
