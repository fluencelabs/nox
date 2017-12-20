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
import fluence.btree.client.core.{ ClientState, SearchTreeRpc }
import fluence.btree.client.merkle.MerklePath
import fluence.btree.client.network.BTreeClientNetwork.{ GetCallback, PutCallback }
import fluence.btree.client.network._
import fluence.crypto.Crypt
import fluence.hash.CryptoHasher
import monix.execution.atomic.Atomic
import org.slf4j.LoggerFactory

import scala.collection.Searching.{ Found, SearchResult }

/**
 * Base implementation of [[SearchTreeRpc]] to calls for a remote MerkleBTree.
 * '''Note that this version is single-thread, you can reads concurrent, but not write.'''
 *
 * @param clientState General state holder for btree client
 * @param network     Network interface for interaction MerkleBTree client with remote btree server.
 * @param keyCrypt    Encrypting/decrypting provider for ''key''
 * @param valueCrypt  Encrypting/decrypting provider for ''value''
 * @param verifier    Arbiter for checking correctness of Btree server responses.
 */
class MerkleBTreeClient[F[_], K, V](
    clientState: Atomic[ClientState],
    network: BTreeClientNetwork[F, K, V],
    keyCrypt: Crypt[K, Array[Byte]],
    valueCrypt: Crypt[V, Array[Byte]],
    verifier: BTreeVerifier
)(implicit ME: MonadError[F, Throwable], ord: Ordering[K]) extends SearchTreeRpc[F, K, V] {

  /** Handler for each ''get'' response from server. */
  private val GetCallback: GetCallback[F, K] = {

    // case when server asks next child
    case (GetState(key: K, mRoot, mPath, _), NextChildSearchResponse(keys, childsChecksums)) ⇒
      processSearch(key, mRoot, mPath, keys, childsChecksums)
        .map {
          case (newMPath, nextRequest) ⇒
            Left(GetState(key, mRoot, newMPath, nextRequest))
        }

    // case when server returns founded leaf
    case (GetState(key: K, mRoot, mPath, _), LeafResponse(keys, values)) ⇒
      val leafProof = verifier.getLeafProof(keys, values)

      if (verifier.checkProof(leafProof, mRoot, mPath)) {
        val value = binarySearch(key, keys) match {
          case Found(idx) ⇒
            log.debug(s"For key=$key was found corresponded value")
            Option(values(idx))
          case _ ⇒
            log.debug(s"For key=$key corresponded value is missing")
            None
        }
        ME.pure(Right(value))
      } else {
        ME.raiseError(new IllegalStateException(
          s"Checksum of leaf didn't pass verifying for key=$key, Leaf($keys, $values)"
        ))
      }
  }

  /** Handler for each ''put'' response from server. */
  private val PutCallback: PutCallback[F, K, V] = {

    // case when server asks next child
    case (PutState(key: K, value: V, mRoot, mPath, _, _), NextChildSearchResponse(keys, childsChecksums)) ⇒
      processSearch(key, mRoot, mPath, keys, childsChecksums)
        .map {
          case (newMPath, nextRequest) ⇒
            Left(PutState(key, value, mRoot, newMPath, nextRequest, None))
        }

    // case when server returns founded leaf
    case (PutState(key: K, value: V, mRoot, mPath, _, _), LeafResponse(keys, values)) ⇒
      val leafProof = verifier.getLeafProof(keys, values)

      if (verifier.checkProof(leafProof, mRoot, mPath)) {

        val searchResult = binarySearch(key, keys)

        val nextRequestToServer = PutRequest(keyCrypt.encrypt(key), valueCrypt.encrypt(value), searchResult)

        val nodeProofBeforePut = leafProof.copy(substitutionIdx = searchResult.insertionPoint)
        val newMPath = mPath.add(nodeProofBeforePut)

        val oldValue = searchResult match {
          case Found(idx) ⇒ Some(values(idx))
          case _          ⇒ None
        }

        ME.pure(Left(PutState(key, value, mRoot, newMPath, nextRequestToServer, oldValue)))
      } else {
        ME.raiseError(new IllegalStateException(
          s"Checksum of leaf didn't pass verifying for key=$key, Leaf($keys, $values)"
        ))
      }

    // case when server asks verify made changes
    case (putState @ PutState(key: K, value: V, mRoot, mPath, putRequest: PutRequest, oldVal), verifyResp: VerifyChanges) ⇒

      verifier
        .newMerkleRoot(mPath, putRequest, verifyResp)
        .map { newMRoot ⇒
          // try to change global client state with new merkle root
          val ClientState(oldMRoot) = clientState.getAndSet(ClientState(newMRoot))
          oldMRoot sameElements mRoot
        } match {

          case None ⇒ // server fail verification
            ME.raiseError(new IllegalStateException(
              s"Server put response didn't pass verifying for state=$putState, newRoot=$verifyResp"
            ))
          case Some(false) ⇒ // client do mutation operations concurrently
            // we can't do anything for restore client state in this situation, the contract was not violated
            ME.raiseError(new IllegalStateException(
              s"Client merkle root was concurrently changed, expectedClientRoot=$mRoot, actualClientRoot=${clientState()}"
            ))
          case Some(true) ⇒ // all is fine, send Confirm to server
            ME.pure(Left(PutState(key, value, mRoot, mPath, Confirm, oldVal)))
        }

    // case when server confirmed receipt
    case (state: PutState[K, V], ConfirmAccepted) ⇒
      ME.pure(Right(state.oldCipherValue))

  }

  /**
   * Finds ''value'' for specified ''key'' in remote MerkleBTree and returns decrypted ''value''.
   *
   * @param key Plain text key
   */
  def get(key: K): F[Option[V]] = {
    network
      .get(GetState(key, clientState()), GetCallback)
      .map(decryptValue)
  }

  /**
   * Save encrypted ''key'' and ''value'' into remote MerkleBTree.
   *
   * @param key   Plain text key
   * @param value Plain text value
   */
  def put(key: K, value: V): F[Option[V]] = {
    network
      .put(PutState(key, value, clientState()), PutCallback)
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
   * @return A tuple with updated mPath and next request to server for resuming of searching
   */
  private def processSearch(
    key: K,
    mRoot: Array[Byte],
    mPath: MerklePath,
    keys: Array[Key],
    childsChecksums: Array[Bytes]
  ): F[(MerklePath, BTreeClientRequest)] = {

    val nodeProof = verifier.getBranchProof(keys, childsChecksums, -1)

    if (verifier.checkProof(nodeProof, mRoot, mPath)) {
      val searchResult = binarySearch(key, keys)
      val nextRequestToServer = ResumeSearchRequest(searchResult.insertionPoint)
      val updatedMPath = mPath.add(nodeProof.copy(substitutionIdx = searchResult.insertionPoint))

      ME.pure(updatedMPath → nextRequestToServer)
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
    initClientState: ClientState,
    network: BTreeClientNetwork[F, K, V],
    keyCrypt: Crypt[K, Array[Byte]],
    valueCrypt: Crypt[V, Array[Byte]],
    cryptoHasher: CryptoHasher[Bytes, Bytes]
  )(implicit ME: MonadError[F, Throwable], ord: Ordering[K]): MerkleBTreeClient[F, K, V] = {
    new MerkleBTreeClient[F, K, V](
      Atomic(initClientState),
      network,
      keyCrypt,
      valueCrypt,
      BTreeVerifier(cryptoHasher)
    )
  }

}
