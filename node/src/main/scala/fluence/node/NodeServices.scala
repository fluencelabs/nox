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

import fluence.crypto.signature.Signer
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.dataset.protocol.{ ContractAllocatorRpc, ContractsApi, ContractsCacheRpc }
import fluence.kad.Kademlia
import fluence.kad.protocol.Key

import scala.language.higherKinds

abstract class NodeServices[F[_], Contract, Contact] {

  def key: Key

  def signer: Signer

  def kademlia: Kademlia[F, Contact]

  def contracts: ContractsApi[F, Contract]

  def contractsCache: ContractsCacheRpc[F, Contract]

  def contractAllocator: ContractAllocatorRpc[F, Contract]

  def datasets: DatasetStorageRpc[F]

}
