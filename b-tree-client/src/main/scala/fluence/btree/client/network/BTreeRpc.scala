package fluence.btree.client.network

import fluence.btree.client.core.PutDetails
import fluence.btree.client.network.BTreeRpc.{ GetCallbacks, PutCallbacks }
import fluence.btree.client.{ Bytes, Key, Value }

trait BTreeRpc[F[_]] {

  def get(callbacks: GetCallbacks[F]): F[Option[Value]]

  def put(callbacks: PutCallbacks[F]): F[Option[Value]]

}

object BTreeRpc {

  trait GetCallbacks[F[_]] {

    // case when server asks next child
    def nextChild(keys: Array[Key], childsChecksums: Array[Bytes]): F[(GetState[F], Int)]

    // case when server returns founded leaf
    def submitLeaf(keys: Array[Key], values: Array[Value]): F[Option[Value]]

  }

  trait PutCallbacks[F[_]] {

    // case when server asks next child
    def nextChild(keys: Array[Key], childsChecksums: Array[Bytes]): F[(PutState[F], Int)]

    // case when server returns founded leaf
    def submitLeaf(keys: Array[Key], values: Array[Value]): F[(PutState[F], PutDetails)]

    // case when server asks verify made changes
    def verifyChanges(serverMerkleRoot: Bytes, wasSplitting: Boolean): F[PutState[F]]

    // case when server confirmed changes persisted
    def changesStored(): F[Option[Value]]

  }

}
