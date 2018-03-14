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

import cats.effect.IO
import fluence.contract.protocol.{ ContractAllocatorRpc, ContractsCacheRpc }
import fluence.crypto.SignAlgo
import fluence.crypto.signature.Signer
import fluence.dataset.protocol.DatasetStorageRpc
import fluence.kad.Kademlia
import fluence.kad.protocol.Key
import fluence.storage.rocksdb.RocksDbStore

import scala.language.higherKinds

abstract class NodeServices[F[_], FS[_], Contract, Contact] {

  def key: Key

  def signer: Signer

  def signAlgo: SignAlgo

  def rocksFactory: RocksDbStore.Factory

  def kademlia: Kademlia[F, Contact]

  def contractsCache: ContractsCacheRpc[F, Contract]

  def contractAllocator: ContractAllocatorRpc[F, Contract]

  def datasets: DatasetStorageRpc[F, FS]

  def close: IO[Unit]

}
