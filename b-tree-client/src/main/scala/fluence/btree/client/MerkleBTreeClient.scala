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

package fluence.btree.client

import cats.MonadError
import cats.syntax.functor._
import fluence.btree.client.MerkleBTreeClient._
import fluence.btree.client.common.BytesOps
import fluence.btree.client.core.{ ClientState, PutDetails, SearchTree }
import fluence.btree.client.merkle.MerklePath
import fluence.btree.client.network.BTreeRpc.{ GetCallbacks, PutCallbacks }
import fluence.btree.client.network._
import fluence.crypto.Crypt
import fluence.hash.CryptoHasher
import monix.execution.atomic.Atomic
import org.slf4j.LoggerFactory

import scala.collection.Searching.{ Found, SearchResult }

/**
 * Base implementation of [[SearchTree]] to calls for a remote MerkleBTree.
 * '''Note that this version is single-thread, you can reads concurrent, but not write.'''
 *
 * @param clientState General state holder for btree client
 * @param bTreeRpc     BTree rpc service.
 * @param keyCrypt    Encrypting/decrypting provider for ''key''
 * @param valueCrypt  Encrypting/decrypting provider for ''value''
 * @param verifier    Arbiter for checking correctness of Btree server responses.
 */
class MerkleBTreeClient[F[_], K, V] private (
    clientState: Atomic[ClientState],
    bTreeRpc: BTreeRpc[F],
    keyCrypt: Crypt[K, Array[Byte]],
    valueCrypt: Crypt[V, Array[Byte]],
    verifier: BTreeVerifier
)(implicit ME: MonadError[F, Throwable], ord: Ordering[K]) extends SearchTree[F, K, V] {

  /**
   * State for each 'Get' request to remote BTree. One ''GetState'' corresponds to one series of round trip requests
   *
   * @param key         The search plain text ''key''. Constant for round trip session.
   * @param merkleRoot  Copy of client merkle root at the beginning of the request. Constant for round trip session.
   * @param merklePath  Tree path traveled on the server. Updatable for round trip session.
   * @param foundValue  Result of this get request. Updatable for round trip session.
   */
  case class GetStateImpl(
      key: K,
      merkleRoot: Array[Byte],
      merklePath: MerklePath,
      foundValue: Option[Value]
  ) extends GetState[F] with GetCallbacks[F] {

    // case when server asks next child
    def nextChild(keys: Array[Key], childsChecksums: Array[Bytes]): F[(GetCallbacks[F], Int)] = {
      processSearch(key, merkleRoot, merklePath, keys, childsChecksums)
        .map {
          case (newMPath, foundIdx) ⇒
            val newSate = this.copy(merklePath = newMPath)
            newSate → foundIdx
        }
    }

    // case when server returns founded leaf
    def submitLeaf(keys: Array[Key], values: Array[Value]): F[Option[Value]] = {
      val leafProof = verifier.getLeafProof(keys, values)
      if (verifier.checkProof(leafProof, merkleRoot, merklePath)) {
        val value = binarySearch(key, keys) match {
          case Found(idx) ⇒
            log.debug(s"For key=$key was found corresponded value")
            Option(values(idx))
          case _ ⇒
            log.debug(s"For key=$key corresponded value is missing")
            None
        }

        ME.pure(value)
      } else {
        ME.raiseError(new IllegalStateException(
          s"Checksum of leaf didn't pass verifying for key=$key, Leaf($keys, $values)"
        ))
      }
    }
  }

  /**
   * State for each 'Put' request to remote BTree. One ''PutState'' corresponds to one series of round trip requests
   *
   * @param key            The search plain text ''key''
   * @param value          Plain text ''value'' to be inserted to server BTree
   * @param merkleRoot     Copy of client merkle root at the beginning of the request
   * @param merklePath     Tree path traveled on the server
   * @param putDetails     All details needed for putting key and value to BTree
   * @param oldCipherValue An old value that will be rewritten or None if key for putting wasn't present in B Tree
   */
  case class PutStateImpl(
      key: K,
      value: V,
      merkleRoot: Bytes,
      merklePath: MerklePath,
      putDetails: Option[PutDetails] = None,
      oldCipherValue: Option[Value] = None
  ) extends PutState[F] with PutCallbacks[F] {

    // case when server asks next child
    override def nextChild(keys: Array[Key], childsChecksums: Array[Bytes]): F[(PutCallbacks[F], Int)] = {
      processSearch(key, merkleRoot, merklePath, keys, childsChecksums)
        .map {
          case (newMPath, foundIdx) ⇒
            val newSate: PutStateImpl = this.copy(merklePath = newMPath)
            newSate → foundIdx
        }
    }

    // case when server returns founded leaf
    override def submitLeaf(keys: Array[Key], values: Array[Value]): F[(PutCallbacks[F], PutDetails)] = {
      val leafProof = verifier.getLeafProof(keys, values)
      if (verifier.checkProof(leafProof, merkleRoot, merklePath)) {
        val searchResult = binarySearch(key, keys)
        val nodeProofBeforePut = leafProof.copy(substitutionIdx = searchResult.insertionPoint)
        val newMPath = merklePath.add(nodeProofBeforePut)
        val prevValue = searchResult match {
          case Found(idx) ⇒
            log.debug(s"For key=$key, value=$value will be perform update, because key isn't present in tree")
            Some(values(idx))
          case _ ⇒
            log.debug(s"For key=$key, value=$value will be perform insert, because key is present in tree")
            None
        }

        val putDetails = PutDetails(keyCrypt.encrypt(key), valueCrypt.encrypt(value), searchResult)
        val newSate = this.copy(merklePath = newMPath, putDetails = Some(putDetails), oldCipherValue = prevValue)

        ME.pure(newSate → putDetails)
      } else {
        ME.raiseError(new IllegalStateException(
          s"Checksum of leaf didn't pass verifying for key=$key, Leaf($keys, $values)"
        ))
      }
    }

    // case when server asks verify made changes
    override def verifyChanges(serverMerkleRoot: Bytes, wasSplitting: Boolean): F[PutCallbacks[F]] = {

      putDetails match {
        case Some(details) ⇒
          verifier
            .newMerkleRoot(merklePath, details, serverMerkleRoot, wasSplitting)
            .map { newMRoot ⇒
              // try to change global client state with new merkle root
              val ClientState(oldMRoot) = clientState.getAndSet(ClientState(newMRoot))
              oldMRoot sameElements merkleRoot
            } match {

              case None ⇒ // server fail verification
                ME.raiseError(new IllegalStateException(
                  s"Server put response didn't pass verifying for state=$this, serverMRoot=$serverMerkleRoot"
                ))
              case Some(false) ⇒ // client do mutation operations concurrently
                // we can't do anything for restore client state in this situation, the contract was not violated
                ME.raiseError(new IllegalStateException(
                  s"Client merkle root was concurrently changed, expectedClientRoot=$merkleRoot, actualClientRoot=${clientState()}"
                ))
              case Some(true) ⇒ // all is fine, send Confirm to server
                ME.pure(this)
            }
        case None ⇒
          ME.raiseError(new IllegalStateException(
            s"Client put details isn't defined, it's should be defined at previous step"
          ))
      }

    }

    // case when server confirmed changes persisted
    override def changesStored(): F[Option[Value]] =
      ME.pure(this.oldCipherValue)
  }

  /**
   * Finds ''value'' for specified ''key'' in remote MerkleBTree and returns decrypted ''value''.
   *
   * @param key Plain text key
   */
  def get(key: K): F[Option[V]] = {
    val state = GetStateImpl(key, BytesOps.copyOf(clientState().merkleRoot), MerklePath.empty, None)

    bTreeRpc.get(state)
      .map(decryptValue)
  }

  /**
   * Save encrypted ''key'' and ''value'' into remote MerkleBTree.
   *
   * @param key   Plain text key
   * @param value Plain text value
   */
  def put(key: K, value: V): F[Option[V]] = {
    val state = PutStateImpl(key, value, BytesOps.copyOf(clientState().merkleRoot), MerklePath.empty, None)

    bTreeRpc.put(state)
      .map(decryptValue)
  }

  def delete(key: K): F[Option[V]] = ???

  private def binarySearch(key: K, keys: Array[Key]): SearchResult = {
    import fluence.crypto.CryptoSearching._
    implicit val decrypt: Key ⇒ K = keyCrypt.decrypt
    keys.binarySearch(key)
  }

  private def decryptValue(encValue: Option[Value]): Option[V] = {
    encValue.map(valueCrypt.decrypt)
  }

  /**
   * Verifies merkle proof for server response, after that search index of next child of branch.
   *
   * @param key             Plain text key
   * @param mRoot           Merkle root for current request
   * @param mPath           Merkle path for current request
   * @param keys            Encrypted keys from server for deciding next tree child
   * @param childsChecksums Checksums of current branch children, for merkle proof checking
   * @return A tuple with updated mPath and searched next tree child index
   */
  private def processSearch(
    key: K,
    mRoot: Array[Byte],
    mPath: MerklePath,
    keys: Array[Key],
    childsChecksums: Array[Bytes]
  ): F[(MerklePath, Int)] = {

    val nodeProof = verifier.getBranchProof(keys, childsChecksums, -1)

    if (verifier.checkProof(nodeProof, mRoot, mPath)) {
      val searchResult = binarySearch(key, keys)
      val updatedMPath = mPath.add(nodeProof.copy(substitutionIdx = searchResult.insertionPoint))

      ME.pure(updatedMPath → searchResult.insertionPoint)
    } else {
      ME.raiseError(new IllegalStateException(
        s"Checksum of branch didn't pass verifying for key=$key, Branch($keys, $childsChecksums)"
      ))
    }
  }

}

object MerkleBTreeClient {

  private val log = LoggerFactory.getLogger(getClass)

  def apply[F[_], K, V](
    initClientState: Option[ClientState],
    bTreeRpc: BTreeRpc[F],
    keyCrypt: Crypt[K, Array[Byte]],
    valueCrypt: Crypt[V, Array[Byte]],
    cryptoHasher: CryptoHasher[Bytes, Bytes]
  )(implicit ME: MonadError[F, Throwable], ord: Ordering[K]): MerkleBTreeClient[F, K, V] = {
    new MerkleBTreeClient[F, K, V](
      Atomic(initClientState.getOrElse(ClientState(Array.emptyByteArray))),
      bTreeRpc,
      keyCrypt,
      valueCrypt,
      BTreeVerifier(cryptoHasher)
    )
  }

}
