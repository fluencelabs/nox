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

import cats.syntax.show._
import fluence.btree.client.MerkleBTreeClient._
import fluence.btree.common.BTreeCommonShow._
import fluence.btree.common._
import fluence.btree.common.merkle.MerklePath
import fluence.btree.protocol.BTreeRpc
import fluence.btree.protocol.BTreeRpc.{ GetCallbacks, PutCallbacks }
import fluence.crypto.cipher.Crypt
import fluence.crypto.hash.CryptoHasher
import monix.eval.{ MVar, Task }
import org.slf4j.LoggerFactory

import scala.collection.Searching.{ Found, SearchResult }

/**
 * Base implementation of [[SearchTree]] to calls for a remote MerkleBTree.
 * '''Note that this version is single-thread for Put operation, and multi-thread for Get operation.'''
 *
 * @param clientState General state holder for btree client
 * @param bTreeRpc    BTree rpc service.
 * @param keyCrypt    Encrypting/decrypting provider for ''key''
 * @param verifier    Arbiter for checking correctness of Btree server responses.
 */
class MerkleBTreeClient[K] private (
    clientState: ClientState,
    bTreeRpc: BTreeRpc[Task],
    keyCrypt: Crypt[K, Array[Byte]],
    verifier: BTreeVerifier
)(implicit ord: Ordering[K]) {

  private val clientStateMVar = MVar(clientState)

  /**
   * State for each 'Get' request to remote BTree. One ''GetState'' corresponds to one series of round trip requests
   *
   * @param key        The search plain text ''key''. Constant for round trip session.
   * @param merkleRoot Copy of client merkle root at the beginning of the request. Constant for round trip session.
   */
  case class GetStateImpl(
      key: K,
      merkleRoot: Array[Byte]
  ) extends GetState[Task] with GetCallbacks[Task] {

    /** Tree path traveled on the server. Updatable for round trip session */
    private val merklePathMVar: MVar[MerklePath] = MVar(MerklePath.empty)

    /** Result of this get request. Updatable for round trip session */
    private val foundValueChecksumMVar: MVar[Option[Hash]] = MVar.empty // todo it should be removed in next time

    // case when server asks next child
    def nextChildIndex(keys: Array[Key], childsChecksums: Array[Bytes]): Task[Int] = {
      merklePathMVar.take.flatMap { mPath ⇒
        log.debug(s"nextChildIndex starts for key=$key, mPath=$mPath, keys=${keys.map(_.show)}")

        processSearch(key, merkleRoot, mPath, keys, childsChecksums)
          .flatMap {
            case (newMPath, foundIdx) ⇒
              merklePathMVar
                .put(newMPath)
                .map(_ ⇒ foundIdx)
          }
      }
    }

    // case when server returns founded leaf
    def submitLeaf(keys: Array[Key], valuesChecksums: Array[Hash]): Task[Option[Int]] = {
      merklePathMVar.take.flatMap { mPath ⇒
        log.debug(s"submitLeaf starts for key=$key, mPath=$mPath, keys=${keys.map(_.show)}")

        val leafProof = verifier.getLeafProof(keys, valuesChecksums)
        if (verifier.checkProof(leafProof, merkleRoot, mPath)) {

          val searchedIdx = binarySearch(key, keys) match {
            case Found(idx) ⇒
              log.debug(s"For key=$key was found corresponded value with idx=$idx")
              Option(idx)
            case _ ⇒
              log.debug(s"For key=$key corresponded value is missing")
              None
          }

          Task.gather(
            Seq(foundValueChecksumMVar.put(searchedIdx.map(valuesChecksums)), merklePathMVar.put(mPath.add(leafProof)))
          ).map(_ ⇒ searchedIdx)
        } else {
          Task.raiseError(new IllegalStateException(
            s"Checksum of leaf didn't pass verifying for key=$key, Leaf($keys, $valuesChecksums)"
          ))
        }
      }
    }

    override def getFoundValue: Task[Option[Hash]] =
      foundValueChecksumMVar.take
  }

  /**
   * State for each 'Put' request to remote BTree. One ''PutState'' corresponds to one series of round trip requests
   *
   * @param key            The search plain text ''key''
   * @param valueChecksum Checksum of encrypted value to be store
   * @param merkleRoot     Copy of client merkle root at the beginning of the request
   */
  case class PutStateImpl private (
      key: K,
      valueChecksum: Hash,
      merkleRoot: Hash
  ) extends PutState[Task] with PutCallbacks[Task] {

    /** Tree path traveled on the server */
    private val merklePathMVar: MVar[MerklePath] = MVar(MerklePath.empty)

    /** All details needed for putting key and value to BTree */
    private val putDetailsMVar: MVar[Option[ClientPutDetails]] = MVar.empty

    /** An old value that will be rewritten or None if key for putting wasn't present in B Tree */
    private val oldCipherValueMVar: MVar[Option[Hash]] = MVar.empty

    /** New valid client merkle root */
    private val newMerkleRoot: MVar[Bytes] = MVar.empty

    // case when server asks next child
    override def nextChildIndex(keys: Array[Key], childsChecksums: Array[Bytes]): Task[Int] = {
      merklePathMVar.take.flatMap { mPath ⇒
        log.debug(s"nextChildIndex starts for key=$key, mPath=$mPath, keys=${keys.map(_.show)}")

        processSearch(key, merkleRoot, mPath, keys, childsChecksums)
          .flatMap {
            case (newMPath, foundIdx) ⇒
              merklePathMVar
                .put(newMPath)
                .map(_ ⇒ foundIdx)
          }
      }
    }

    // case when server returns founded leaf
    override def putDetails(keys: Array[Key], values: Array[Hash]): Task[ClientPutDetails] = {
      merklePathMVar.take.flatMap { mPath ⇒
        log.debug(s"putDetails starts for key=$key, mPath=$mPath, keys=${keys.map(_.show)}")

        val leafProof = verifier.getLeafProof(keys, values)
        if (verifier.checkProof(leafProof, merkleRoot, mPath)) {
          val searchResult = binarySearch(key, keys)
          val nodeProofWithIdx = leafProof.copy(substitutionIdx = searchResult.insertionPoint)
          val prevValue = searchResult match {
            case Found(idx) ⇒
              log.debug(s"Start update for key=$key, valHash=$valueChecksum, because key is present in tree by idx=$idx")
              Some(values(idx))
            case _ ⇒
              log.debug(s"Start insert for key=$key, valHash=$valueChecksum, because key isn't present in tree")
              None
          }
          val putDetails = ClientPutDetails(keyCrypt.encrypt(key), valueChecksum, searchResult)

          for {
            _ ← putDetailsMVar.put(Some(putDetails))
            _ ← oldCipherValueMVar.put(prevValue)
            _ ← merklePathMVar.put(mPath.add(nodeProofWithIdx))
          } yield putDetails

        } else {
          Task.raiseError(new IllegalStateException(
            s"Checksum of leaf didn't pass verifying for key=$key, Leaf($keys, $values)"
          ))
        }
      }
    }

    // case when server asks verify made changes
    override def verifyChanges(serverMRoot: Bytes, wasSplitting: Boolean): Task[Unit] = {
      for {
        mPath ← merklePathMVar.read
        optDetails ← putDetailsMVar.read
        _ ← {
          log.debug(s"verifyChanges starts for key=$key, mPath=$mPath, details=$optDetails, serverMRoot=$serverMRoot")

          optDetails match {
            case Some(details) ⇒
              Task(verifier.newMerkleRoot(mPath, details, serverMRoot, wasSplitting))
                .flatMap {
                  case None ⇒ // server was failed verification
                    Task.raiseError(new IllegalStateException(
                      s"Server 'put response' didn't pass verifying for state=$this, serverMRoot=$serverMRoot"
                    ))
                  case Some(newMRoot) ⇒ // all is fine, send Confirm to server
                    newMerkleRoot.put(newMRoot)
                }
            case None ⇒
              Task.raiseError(new IllegalStateException(
                s"Client put details isn't defined, it's should be defined at previous step"
              ))
          }
        }
      } yield ()

    }

    // case when server confirmed changes persisted
    override def changesStored(): Task[Unit] = {
      // change global client state with new merkle root
      newMerkleRoot.take
        .flatMap { newMRoot ⇒
          log.debug(s"changesStored starts for key=$key, newMRoot=$newMRoot")

          clientStateMVar.put(ClientState(newMRoot))
        }
    }

    // return old value checksum for search key
    override def getOldValue: Task[Option[Hash]] =
      oldCipherValueMVar.read
  }

  /**
   * Finds ''value'' for specified ''key'' in remote MerkleBTree and returns decrypted ''value''.
   *
   * @param key Plain text key
   */
  def get(key: K): Task[Option[Hash]] = { // todo return type will be changed in next task
    log.debug(s"Get starts for key=$key")

    for {
      clientState ← clientStateMVar.read
      state = GetStateImpl(key, BytesOps.copyOf(clientState.merkleRoot))
      _ ← bTreeRpc.get(state)
      result ← state.getFoundValue
    } yield result

  }

  /**
   * Save encrypted ''key'' and ''value'' into remote MerkleBTree.
   *
   * @param key             Plain text key
   * @param valueChecksum  Checksum of encrypted value to be store
   */
  def put(key: K, valueChecksum: Hash): Task[Option[Hash]] = { // todo return type will be changed in next task
    log.debug(s"Put starts put for key=$key, value=$valueChecksum")

    val res = for {
      clientState ← clientStateMVar.take
      state = PutStateImpl(key, valueChecksum, BytesOps.copyOf(clientState.merkleRoot))
      _ ← bTreeRpc.put(state) // return old value to clientStateMVar
      result ← state.getOldValue
    } yield result
    res.doOnFinish {
      // in error case we should return old value of clientState back
      case Some(e) ⇒ clientStateMVar.put(clientState)
      case _       ⇒ Task(())
    }
  }

  private def binarySearch(key: K, keys: Array[Key]): SearchResult = {
    import fluence.crypto.cipher.CryptoSearching._
    implicit val decrypt: Key ⇒ K = keyCrypt.decrypt
    keys.binarySearch(key)
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
  ): Task[(MerklePath, Int)] = {

    val nodeProof = verifier.getBranchProof(keys, childsChecksums, -1)

    if (verifier.checkProof(nodeProof, mRoot, mPath)) {
      val searchResult = binarySearch(key, keys)
      val updatedMPath = mPath.add(nodeProof.copy(substitutionIdx = searchResult.insertionPoint))

      Task(updatedMPath → searchResult.insertionPoint)
    } else {
      Task.raiseError(new IllegalStateException(
        s"Checksum of branch didn't pass verifying for key=$key, Branch($keys, $childsChecksums)"
      ))
    }
  }

}

object MerkleBTreeClient {

  private val log = LoggerFactory.getLogger(getClass)

  def apply[K](
    initClientState: Option[ClientState],
    bTreeRpc: BTreeRpc[Task],
    keyCrypt: Crypt[K, Array[Byte]],
    cryptoHasher: CryptoHasher[Bytes, Bytes]
  )(implicit ord: Ordering[K]): MerkleBTreeClient[K] = {
    new MerkleBTreeClient[K](
      initClientState.getOrElse(ClientState(Array.emptyByteArray)),
      bTreeRpc,
      keyCrypt,
      BTreeVerifier(cryptoHasher)
    )
  }

  /**
   * Global state of BTree client.
   */
  case class ClientState(merkleRoot: Bytes)

}
