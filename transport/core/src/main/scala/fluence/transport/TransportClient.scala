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

import fluence.kad.protocol.Contact
import shapeless.{ops, HList}

trait TransportClient[CL <: HList] {

  /**
    * Returns a service stub for a particular contact.
    *
    * @param contact To open service for
    * @param sel     Implicit selector from HList
    * @tparam T Type of the service
    * @return
    */
  def service[T](contact: Contact)(implicit sel: ops.hlist.Selector[CL, T]): T
}
