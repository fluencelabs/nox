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

package fluence.node.core

import java.time.Clock

import cats.effect.IO
import cats.~>
import com.typesafe.config.Config
import fluence.client.core.config.KademliaConfigParser
import fluence.contract.BasicContract
import fluence.contract.node.cache.ContractRecord
import fluence.contract.node.{ContractAllocator, ContractsCache}
import fluence.contract.protocol.{ContractAllocatorRpc, ContractsCacheRpc}
import fluence.crypto.SignAlgo
import fluence.crypto.hash.CryptoHasher
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.Signer
import fluence.dataset.node.Datasets
import fluence.dataset.protocol.DatasetStorageRpc
import fluence.kad.protocol.{Contact, KademliaRpc, Key}
import fluence.kad.{Kademlia, KademliaMVar}
import fluence.storage.KVStore
import fluence.storage.rocksdb.RocksDbStore
import fluence.transport.TransportSecurity
import monix.eval.Task
import monix.execution.Scheduler
import monix.reactive.Observable
import scodec.bits.ByteVector

import scala.concurrent.duration._
import scala.language.higherKinds

object NodeComposer {

  type Services = NodeServices[Task, Observable, BasicContract, Contact]

  /**
   * Builds all node services.
   *
   * @param keyPair Node keyPair with private and public keys
   * @param contact Node contact
   * @param algo    Crypto algorithm for generation keys, creating signers and checkers
   * @param cryptoHasher Crypto hash function provider
   * @param kadClient    Provider for creating KademliaRpc for specific contact
   * @param config       Global typeSafe config for node
   * @param acceptLocal If true, local addresses will be accepted; should be used only in testing (will be removed later)
   * @param clock        Physic time provider
   */
  def services(
    keyPair: KeyPair,
    contact: Contact,
    algo: SignAlgo,
    cryptoHasher: CryptoHasher[Array[Byte], Array[Byte]],
    kadClient: Contact ⇒ KademliaRpc[Contact],
    config: Config,
    acceptLocal: Boolean, // todo move acceptLocal to node config, and remove from here
    clock: Clock
  )(implicit scheduler: Scheduler): IO[Services] =
    for {
      nodeKey ← Key.fromKeyPair[IO](keyPair)
      kadConf ← KademliaConfigParser.readKademliaConfig[IO](config)
      rocksDbFactory = new RocksDbStore.Factory
      contractsCacheStore ← ContractsCacheStore(config, dirName ⇒ rocksDbFactory[IO](dirName, config))
    } yield
      new NodeServices[Task, Observable, BasicContract, Contact] {
        override val key: Key = nodeKey

        override def rocksFactory: RocksDbStore.Factory = rocksDbFactory

        override val signer: Signer = algo.signer(keyPair)

        override val signAlgo: SignAlgo = algo

        import algo.checker

        private object taskToIO extends (Task ~> IO) {
          override def apply[A](fa: Task[A]): IO[A] = fa.toIO(scheduler)
        }

        override lazy val kademlia: Kademlia[Task, Contact] = KademliaMVar(
          nodeKey,
          IO.pure(contact),
          kadClient,
          kadConf,
          TransportSecurity.canBeSaved[IO, Contact](nodeKey, acceptLocal = acceptLocal)
        )

        override lazy val contractsCache: ContractsCacheRpc[BasicContract] =
          new ContractsCache[Task, BasicContract](
            nodeId = nodeKey,
            storage = contractsCacheStore,
            cacheTtl = 1.day,
            clock,
            taskToIO
          )

        override lazy val contractAllocator: ContractAllocatorRpc[BasicContract] =
          new ContractAllocator[Task, BasicContract](
            nodeId = nodeKey,
            storage = contractsCacheStore,
            createDataset = _ ⇒ Task(true), // TODO: dataset creation
            checkAllocationPossible = _ ⇒ Task(true), // TODO: check allocation possible
            signer = signer,
            clock,
            toIO = taskToIO
          )

        override lazy val datasets: DatasetStorageRpc[Task, Observable] =
          new Datasets(
            config,
            rocksFactory,
            cryptoHasher,
            // Return contract version, if current node participates in it
            servesDatasetFn(contractsCacheStore, key),
            // Update contract's version and merkle root, if newVersion = currentVersion+1
            updateContractFn(clock, contractsCacheStore),
          )

        // Register everything that should be closed or cleaned up on shutdown here
        override def close: IO[Unit] =
          rocksFactory.close
      }

  /**
   * Creates function that returns contract state, if current node participates in it, and None otherwise
   */
  private[core] def servesDatasetFn(
    contractsCacheStore: KVStore[Task, Key, ContractRecord[BasicContract]],
    nodeKey: Key
  ): Key ⇒ Task[Option[Long]] =
    contractKey ⇒ {
      contractsCacheStore
        .get(contractKey)
        .map { contract ⇒
          Option(contract.contract.executionState.version)
            .filter(_ ⇒ contract.contract.participants.contains(nodeKey))
        }
    }

  /**
   * Creates function for updating contract after dataset changed.
   * Function updates contract's version and merkle root
   */
  private[core] def updateContractFn(
    clock: Clock,
    contractsCacheStore: KVStore[Task, Key, ContractRecord[BasicContract]]
  ): (Key, ByteVector, Long) ⇒ Task[Unit] =
    (datasetId, merkleRoot, datasetVer) ⇒ { // todo add client sign
      for {
        contract ← contractsCacheStore.get(datasetId)
        updatedContract ← {
          if (contract.contract.executionState.version == datasetVer)
            Task {
              contract.copy(
                contract = contract.contract.copy(
                  executionState = BasicContract.ExecutionState(
                    version = datasetVer + 1, // increment expected client version by one, because dataset change state
                    merkleRoot = merkleRoot // new merkle root after made changes
                  )
                ),
                lastUpdated = clock.instant()
              )
            } else
            Task.raiseError(
              new IllegalStateException(
                s"Inconsistent state for contract $datasetId, contract version=${contract.contract.executionState.version}," +
                  s" asking for update to $datasetVer"
              )
            )
        }

        _ ← contractsCacheStore.put(datasetId, updatedContract)
      } yield () // TODO: schedule broadcasting the contract to kademlia

    }

}
