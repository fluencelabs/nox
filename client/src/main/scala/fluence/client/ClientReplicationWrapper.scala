package fluence.client

import fluence.dataset.client.ClientDatasetStorageApi
import fluence.kad.protocol.Contact
import monix.eval.Task

import scala.language.higherKinds

/**
 * Wrapper with dataset replication. Allows writes data to all nodes and reads data if at least one node is alive.
 * This is naive implementation of replication and it will be removed in future.
 */
class ClientReplicationWrapper[K, V](
    datasetReplicas: List[(ClientDatasetStorageApi[Task, K, V], Contact)]
) extends ClientDatasetStorageApi[Task, K, V] with slogging.LazyLogging {

  private val replicationFactor = datasetReplicas.size
  /**
   * Gets stored value for specified key from first server.
   *
   * @param key The key retrieve the value.
   * @return returns found value, None if nothing was found.
   */
  override def get(key: K): Task[Option[V]] = {

    def getRec(replicas: List[(ClientDatasetStorageApi[Task, K, V], Contact)]): Task[Option[V]] = {
      val (store, contact) = replicas.head
      logger.info(s"Reading key=$key from ${contact.ip}:${contact.grpcPort}")
      store.get(key).onErrorHandleWith { e ⇒
        logger.warn(s"Can't get value from ${contact.ip}:${contact.grpcPort} for key=$key, cause=$e")
        if (replicas.tail.isEmpty)
          Task.raiseError[Option[V]](e)
        else {
          getRec(replicas.tail)
        }
      }
    }

    getRec(datasetReplicas.toList)
  }

  /**
   * Puts key value pair (K, V) on each server, waits all responses and return first result.
   *
   * @param key   The specified key to be inserted
   * @param value The value associated with the specified key
   * @return returns old value if old value was overridden, None otherwise.
   */
  override def put(key: K, value: V): Task[Option[V]] = {
    Task.sequence(
      datasetReplicas
        .map { case (store, _) ⇒ store.put(key, value) }
    ).map { seq ⇒
        logger.info(s"$key and $value was written to $replicationFactor nodes")
        seq.head // at least one server should be here, we can't allocate contract without participants
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
