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

package fluence.client.core

import fluence.dataset.client.ClientDatasetStorageApi
import fluence.kad.protocol.Contact
import monix.eval.Task
import monix.reactive.Observable

import scala.language.higherKinds

/**
 * Wrapper with dataset replication. Allows writes data to all nodes and reads data if at least one node is alive.
 * This is naive implementation of replication and it will be removed in future.
 */
class ClientReplicationWrapper[K, V](
    datasetReplicas: List[(ClientDatasetStorageApi[Task, Observable, K, V], Contact)]
) extends ClientDatasetStorageApi[Task, Observable, K, V] with slogging.LazyLogging {

  private val replicationFactor = datasetReplicas.size
  /**
   * Gets stored value for specified key from first server.
   *
   * @param key The key retrieve the value.
   * @return returns found value, None if nothing was found.
   */
  override def get(key: K): Task[Option[V]] = {

    def getRec(replicas: List[(ClientDatasetStorageApi[Task, Observable, K, V], Contact)]): Task[Option[V]] = {
      val (store, contact) = replicas.head
      logger.info(s"Reading key=$key from ${contact.addr}:${contact.grpcPort}")
      store.get(key).onErrorHandleWith { e ⇒
        logger.warn(s"Can't get value from ${contact.addr}:${contact.grpcPort} for key=$key, cause=$e")
        if (replicas.tail.isEmpty)
          Task.raiseError[Option[V]](e)
        else {
          getRec(replicas.tail)
        }
      }
    }

    getRec(datasetReplicas)
  }

  /**
   * Fetches stored key-value pairs for specified key range as stream.
   *
   * @param from Plain text key, start of range.
   * @param to   Plain text key, end of range.
   * @return returns stream of found key-value pairs.
   */
  override def range(from: K, to: K): Observable[(K, V)] = {

    def rangeRec(replicas: List[(ClientDatasetStorageApi[Task, Observable, K, V], Contact)]): Observable[(K, V)] = {
      val (store, contact) = replicas.head
      logger.info(s"Reading key=$from from ${contact.addr}:${contact.grpcPort}")
      store.range(from, to).onErrorHandleWith { e ⇒
        logger.warn(s"Can't get value from ${contact.addr}:${contact.grpcPort} for key=$from, cause=$e")
        if (replicas.tail.isEmpty)
          Observable.raiseError[(K, V)](e)
        else {
          rangeRec(replicas.tail)
        }
      }
    }

    rangeRec(datasetReplicas)
  }

  /**
   * Puts key value pair (K, V) on each server, waits all responses and return first result.
   *
   * @param key   The specified key to be inserted
   * @param value The value associated with the specified key
   * @return returns old value if old value was overridden, None otherwise.
   */
  override def put(key: K, value: V): Task[Option[V]] = {
    for {
      //check that all datasets is available, for demo purpose only
      getRes ← Task.sequence(
        datasetReplicas
          .map { case (store, _) ⇒ store.get(key) }
      )
      res ← Task.sequence(
        datasetReplicas
          .map { case (store, _) ⇒ store.put(key, value) }
      )
    } yield {
      logger.info(s"$key and $value was written to $replicationFactor nodes")
      res.head // at least one server should be here, we can't allocate contract without participants
    }
  }

  /**
   * Removes pair (K, V) for specified key.
   *
   * @param key The key to delete within database
   * @return returns old value that was deleted, None if nothing was deleted.
   */
  override def remove(key: K): Task[Option[V]] = ???

}
