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

import java.time.Instant

import cats.effect.IO
import com.typesafe.config.Config
import fluence.client.core.config.KademliaConfigParser
import fluence.contract.BasicContract
import fluence.contract.node.{ ContractAllocator, ContractsCache }
import fluence.contract.protocol.{ ContractAllocatorRpc, ContractsCacheRpc }
import fluence.crypto.SignAlgo
import fluence.crypto.hash.CryptoHasher
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.Signer
import fluence.dataset.node.Datasets
import fluence.dataset.protocol.DatasetStorageRpc
import fluence.kad.protocol.{ Contact, KademliaRpc, Key }
import fluence.kad.{ Kademlia, KademliaMVar }
import fluence.storage.rocksdb.RocksDbStore
import fluence.transport.TransportSecurity
import monix.eval.Task

import scala.concurrent.duration._
import scala.language.higherKinds

object NodeComposer {

  type Services = NodeServices[Task, BasicContract, Contact]

  def services(
    keyPair: KeyPair,
    contact: Contact,
    algo: SignAlgo,
    cryptoHasher: CryptoHasher[Array[Byte], Array[Byte]],
    kadClient: Contact ⇒ KademliaRpc[Task, Contact],
    config: Config,
    acceptLocal: Boolean
  ): IO[Services] =
    for {
      k ← Key.fromKeyPair[IO](keyPair)
      kadConf ← KademliaConfigParser.readKademliaConfig[IO](config)
      rocksDbFactory = new RocksDbStore.Factory
      contractsCacheStore ← ContractsCacheStore(config, dirName ⇒ rocksDbFactory[IO](dirName, config))
    } yield new NodeServices[Task, BasicContract, Contact] {
      override val key: Key = k

      override def rocksFactory: RocksDbStore.Factory = rocksDbFactory

      override val signer: Signer = algo.signer(keyPair)

      override val signAlgo: SignAlgo = algo

      import algo.checker

      override lazy val kademlia: Kademlia[Task, Contact] = KademliaMVar(
        k,
        Task.now(contact),
        kadClient,
        kadConf,
        TransportSecurity.canBeSaved[Task](k, acceptLocal = acceptLocal)
      )

      override lazy val contractsCache: ContractsCacheRpc[Task, BasicContract] =
        new ContractsCache[Task, BasicContract](
          nodeId = k,
          storage = contractsCacheStore,
          cacheTtl = 1.day
        )

      override lazy val contractAllocator: ContractAllocatorRpc[Task, BasicContract] =
        new ContractAllocator[Task, BasicContract](
          nodeId = k,
          storage = contractsCacheStore,
          createDataset = _ ⇒ Task.unit, // TODO: dataset creation
          checkAllocationPossible = _ ⇒ Task.unit, // TODO: check allocation possible
          signer = signer
        )

      override lazy val datasets: DatasetStorageRpc[Task] =
        new Datasets(
          config,
          rocksFactory,
          cryptoHasher,

          // Return contract version, if current node participates in it
          contractsCacheStore.get(_)
            .map(c ⇒ Option(c.contract.executionState.version).filter(_ ⇒ c.contract.participants.contains(k))),

          // Update contract's version and merkle root, if newVersion = currentVersion+1
          (dsId, v, mr) ⇒ {
            // todo should be moved to separate class and write unit tests
            for {
              c ← contractsCacheStore.get(dsId)
              _ ← if (c.contract.executionState.version == v - 1) Task.unit
              else Task.raiseError(new IllegalStateException(s"Inconsistent state for contract $dsId, contract version=${c.contract.executionState.version}, asking for update to $v"))

              u = c.copy(
                contract = c.contract.copy(
                  executionState = BasicContract.ExecutionState(
                    version = v,
                    merkleRoot = mr
                  )
                ),
                lastUpdated = Instant.now()
              )
              _ ← contractsCacheStore.put(dsId, u)
            } yield () // TODO: schedule broadcasting the contract to kademlia

          }
        )

      // Register everything that should be closed or cleaned up on shutdown here
      override def close: IO[Unit] =
        rocksFactory.close
    }

}
