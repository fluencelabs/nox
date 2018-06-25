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

import cats.data.OptionT
import cats.effect.IO
import com.typesafe.config.Config
import fluence.client.core.config.KademliaConfigParser
import fluence.contract.BasicContract
import fluence.contract.node.cache.ContractRecord
import fluence.contract.node.{ContractAllocator, ContractsCache}
import fluence.contract.protocol.{ContractAllocatorRpc, ContractsCacheRpc}
import fluence.crypto.signature.SignAlgo.CheckerFn
import fluence.crypto.signature.{SignAlgo, Signer}
import fluence.crypto.{Crypto, KeyPair}
import fluence.dataset.node.DatasetNodeStorage.DatasetChanged
import fluence.dataset.node.Datasets
import fluence.dataset.protocol.DatasetStorageRpc
import fluence.kad.protocol.{Contact, ContactSecurity, KademliaRpc, Key}
import fluence.kad.{Kademlia, KademliaMVar}
import fluence.kvstore.ReadWriteKVStore
import fluence.kvstore.rocksdb.RocksDbKVStore
import fluence.storage.rocksdb.RocksDbStore
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
   * @param keyPair      Node keyPair with private and public keys
   * @param contact      Node contact
   * @param algo         Crypto algorithm for generation keys, creating signers and checkers
   * @param cryptoHasher Crypto hash function provider
   * @param kadClient    Provider for creating KademliaRpc for specific contact
   * @param config       Global typeSafe config for node
   * @param acceptLocal  If true, local addresses will be accepted; should be used only in testing (will be removed later)
   * @param clock        Physic time provider
   * @param globalPool   Global thread pool for all business logic.
   * @param ioPool       Thread pool for all input\output operation. If you need separate thread pool
   *                      for network or disc operation feel free to create it.
   */
  def services(
    keyPair: KeyPair,
    contact: Contact,
    algo: SignAlgo,
    cryptoHasher: Crypto.Hasher[Array[Byte], Array[Byte]],
    kadClient: Contact ⇒ KademliaRpc[Contact],
    config: Config,
    acceptLocal: Boolean, // todo move acceptLocal to node config, and remove from here
    clock: Clock,
    globalPool: Scheduler,
    ioPool: Scheduler
  ): IO[Services] =
    for {
      nodeKey ← Key.fromKeyPair.runF[IO](keyPair)
      kadConf ← KademliaConfigParser.readKademliaConfig[IO](config)
      rocksDbFactoryOld = new RocksDbStore.Factory // todo will be removed later
      rocksDbFactoryNew = RocksDbKVStore.getFactory(threadPool = ioPool)
      contractsCacheStore ← ContractsCacheStore.applyOld(config, name ⇒ rocksDbFactoryNew(name, config))
    } yield
      new NodeServices[Task, Observable, BasicContract, Contact] {
        override val key: Key = nodeKey

        override def rocksFactory: RocksDbStore.Factory = rocksDbFactoryOld

        override val signer: Signer = algo.signer(keyPair)

        override val signAlgo: SignAlgo = algo

        import algo.checker

        override lazy val kademlia: Kademlia[Task, Contact] = KademliaMVar(
          nodeKey,
          IO.pure(contact),
          kadClient,
          kadConf,
          ContactSecurity.check[IO](nodeKey, acceptLocal = acceptLocal)
        )

        override lazy val contractsCache: ContractsCacheRpc[BasicContract] =
          new ContractsCache[Task, BasicContract](
            nodeId = nodeKey,
            storage = contractsCacheStore,
            cacheTtl = 1.day,
            clock
          )

        override lazy val contractAllocator: ContractAllocatorRpc[BasicContract] =
          new ContractAllocator[BasicContract](
            nodeId = nodeKey,
            storage = contractsCacheStore,
            createDataset = _ ⇒ IO(true), // TODO: dataset creation
            checkAllocationPossible = _ ⇒ IO(true), // TODO: check allocation possible
            signer = signer,
            clock
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
          )(globalPool)

        // Register everything that should be closed or cleaned up on shutdown here
        override def close: IO[Unit] =
          for {
            _ ← rocksDbFactoryNew.close // todo will be removed later, when btree will be with new KVStore
            _ ← rocksFactory.close
          } yield ()
      }

  /**
   * Creates function that returns contract state, if current node participates in it, and None otherwise
   */
  private[core] def servesDatasetFn(
    contractsCacheStore: ReadWriteKVStore[Key, ContractRecord[BasicContract]],
    nodeKey: Key
  ): Key ⇒ Task[Option[Long]] =
    contractKey ⇒ {
      contractsCacheStore
        .get(contractKey)
        .runF[Task]
        .map { contract ⇒
          contract
            .filter(_.contract.participants.contains(nodeKey))
            .map(_.contract.executionState.version)
        }
    }

  /**
   * Creates function for updating contract after dataset changed.
   * Function updates contract's version and merkle root
   */
  private[core] def updateContractFn(
    clock: Clock,
    contractsCacheStore: ReadWriteKVStore[Key, ContractRecord[BasicContract]]
  )(implicit checkerFn: CheckerFn): (Key, DatasetChanged) ⇒ Task[Unit] =
    (datasetId, datasetChanged) ⇒ {
      val DatasetChanged(newMerkleRoot, newDatasetVer, clientSignature) = datasetChanged

      for {
        contract ← OptionT(contractsCacheStore.get(datasetId).runF[Task])
          .getOrElseF(Task.raiseError(new RuntimeException(s"For dataset=$datasetId wasn't found correspond contract")))

        _ ← checkerFn(contract.contract.publicKey)
          .check[Task](clientSignature, ByteVector.fromLong(newDatasetVer) ++ newMerkleRoot)
          .value
          .flatMap {
            case Left(err) ⇒ Task.raiseError(err)
            case Right(value) ⇒ Task(value)
          }
        updatedContract ← {
          if (contract.contract.executionState.version == newDatasetVer - 1)
            Task {
              contract.copy(
                contract = contract.contract.copy(
                  executionState = BasicContract.ExecutionState(
                    version = newDatasetVer,
                    merkleRoot = newMerkleRoot // new merkle root after made changes
                  ),
                  // node updates contract with client seal for new exec state
                  executionSeal = clientSignature
                ),
                lastUpdated = clock.instant()
              )
            } else
            Task.raiseError(
              new IllegalStateException(
                s"Inconsistent state for contract $datasetId, contract version=${contract.contract.executionState.version}," +
                  s" asking for update to $newDatasetVer"
              )
            )
        }

        _ ← contractsCacheStore.put(datasetId, updatedContract).runF[Task]
      } yield () // TODO: schedule broadcasting the contract to kademlia

    }

}
