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

import fluence.btree.client.MerkleBTreeClient._
import fluence.btree.common.merkle.MerklePath
import fluence.btree.core.{ClientPutDetails, Hash, Key}
import fluence.btree.protocol.BTreeRpc.{PutCallbacks, SearchCallback}
import fluence.crypto.cipher.Crypt
import fluence.crypto.hash.CryptoHasher
import fluence.crypto.signature.Signer
import monix.eval.{MVar, Task}
import scodec.bits.ByteVector

import scala.collection.Searching.{Found, SearchResult}

/**
 * Base implementation of [[MerkleBTreeClientApi]] to calls for a remote MerkleBTree.
 * '''Note that this version is single-thread for Put operation, and multi-thread for Get operation.'''
 *
 * @param initClientState General state holder for btree client. For new dataset should be ''None''
 * @param keyCrypt        Encrypting/decrypting provider for ''key''
 * @param verifier        Arbiter for checking correctness of Btree server responses.
 * @param signer          Algorithm to produce signatures. Used for sealing execState by contract owner
 *
 * @param ord              The ordering to be used to compare keys.
 *
 * @tparam K The type of keys
 */
class MerkleBTreeClient[K] private (
  initClientState: ClientState,
  keyCrypt: Crypt[Task, K, Key],
  verifier: BTreeVerifier,
  signer: Signer
)(implicit ord: Ordering[K])
    extends MerkleBTreeClientApi[Task, K] with slogging.LazyLogging {

  private val clientStateMVar = MVar(initClientState).memoize

  /**
   * State for each search ('Get', 'Range', 'Delete') request to remote BTree.
   * One ''SearchState'' corresponds to one series of round trip requests.
   *
   * @param key        The search plain text ''key''. Constant for round trip session.
   * @param merkleRoot Copy of client merkle root at the beginning of the request. Constant for round trip session.
   */
  case class SearchStateImpl(
    key: K,
    merkleRoot: Hash
  ) extends SearchState[Task] with SearchCallback[Task] {

    /** Tree path traveled on the server. Updatable for round trip session */
    private val merklePathMVar: Task[MVar[MerklePath]] = MVar(MerklePath.empty).memoize

    // case when server asks next child
    override def nextChildIndex(keys: Array[Key], childsChecksums: Array[Hash]): Task[Int] =
      for {
        mvar ← merklePathMVar
        mPath ← mvar.take
        _ = logger.debug(s"nextChildIndex starts for key=$key, mPath=$mPath, keys=${keys.mkString(",")}")
        up ← processSearch(key, merkleRoot, mPath, keys, childsChecksums)
        (newMPath, foundIdx) = up
        _ ← mvar.put(newMPath)
      } yield foundIdx

    // case when server returns founded leaf, this leaf either contains key, or key may be inserted in this leaf
    override def submitLeaf(keys: Array[Key], valuesChecksums: Array[Hash]): Task[SearchResult] =
      for {
        mvar ← merklePathMVar
        mPath ← mvar.take
        leafProof = {
          logger.debug(s"submitLeaf starts for key=$key, mPath=$mPath, keys=${keys.mkString(",")}")
          verifier.getLeafProof(keys, valuesChecksums)
        }
        result ← if (verifier.checkProof(leafProof, merkleRoot, mPath)) {
          binarySearch(key, keys).flatMap { searchResult ⇒
            logger.debug(s"Searching for key=$key returns $searchResult")
            mvar.put(mPath.add(leafProof)).map(_ ⇒ searchResult)
          }
        } else {
          // TODO: return business error as value
          Task.raiseError(
            new IllegalStateException(
              s"Checksum of leaf didn't pass verifying for key=$key, Leaf(${keys.mkString(",")}, " +
                s"${valuesChecksums.mkString(",")})"
            )
          )
        }
      } yield result

    override def recoverState(): Task[Unit] = {
      logger.debug(s"Recover client state for search with key=$key mRoot=$merkleRoot")
      clientStateMVar.flatMap(_.put(ClientState(merkleRoot)))
    }

  }

  /**
   * State for each 'Put' request to remote BTree. One ''PutState'' corresponds to one series of round trip requests
   *
   * @param key            The search plain text ''key''
   * @param valueChecksum Checksum of encrypted value to be store
   * @param version        Dataset version expected to the client
   * @param merkleRoot     Copy of client merkle root at the beginning of the request
   */
  case class PutStateImpl private (
    key: K,
    valueChecksum: Hash,
    version: Long,
    merkleRoot: Hash
  ) extends PutState[Task] with PutCallbacks[Task] {

    /** Tree path traveled on the server */
    private val merklePathMVar: Task[MVar[MerklePath]] = MVar(MerklePath.empty).memoize

    /** All details needed for putting key and value to BTree */
    private val putDetailsMVar: Task[MVar[Option[ClientPutDetails]]] = MVar.empty.memoize

    /** New valid client merkle root */
    private val newMerkleRoot: Task[MVar[Hash]] = MVar.empty.memoize

    // case when server asks next child
    override def nextChildIndex(keys: Array[Key], childsChecksums: Array[Hash]): Task[Int] =
      for {
        mvar ← merklePathMVar
        mPath ← mvar.take
        _ = logger.debug(s"nextChildIndex starts for key=$key, mPath=$mPath, keys=${keys.mkString(",")}")
        searched ← processSearch(key, merkleRoot, mPath, keys, childsChecksums)
        (newMPath, foundIdx) = searched
        _ ← mvar.put(newMPath)
      } yield foundIdx

    // case when server returns founded leaf
    override def putDetails(keys: Array[Key], values: Array[Hash]): Task[ClientPutDetails] =
      merklePathMVar.flatMap { mvar ⇒
        mvar.take.flatMap { mPath ⇒
          logger.debug(s"putDetails starts for key=$key, mPath=$mPath, keys=${keys.mkString(",")}")

          val leafProof = verifier.getLeafProof(keys, values)
          if (verifier.checkProof(leafProof, merkleRoot, mPath)) {

            for {
              searchResult ← binarySearch(key, keys)
              nodeProofWithIdx = leafProof.copy(substitutionIdx = searchResult.insertionPoint)
              encrypted ← keyCrypt.encrypt(key)
              putDetails = ClientPutDetails(encrypted, valueChecksum, searchResult)
              putMvar ← putDetailsMVar
              _ ← putMvar.put(Some(putDetails))
              _ ← mvar.put(mPath.add(nodeProofWithIdx))
            } yield putDetails

          } else {
            Task.raiseError(
              new IllegalStateException(
                s"Checksum of leaf didn't pass verifying for key=$key, Leaf(${keys.mkString(",")}, ${keys.mkString(",")})"
              )
            )
          }
        }
      }

    // case when server asks verify made changes
    override def verifyChanges(serverMRoot: Hash, wasSplitting: Boolean): Task[ByteVector] = {
      for {
        mPath ← merklePathMVar.flatMap(_.read)
        optDetails ← putDetailsMVar.flatMap(_.read)
        signedNewState ← {
          logger.debug(
            s"verifyChanges starts for key=$key, mPath=$mPath, details=$optDetails, serverMRoot=$serverMRoot"
          )

          optDetails match {
            case Some(details) ⇒
              Task(verifier.newMerkleRoot(mPath, details, serverMRoot, wasSplitting)).flatMap {
                case None ⇒ // server was failed verification
                  Task.raiseError(
                    new IllegalStateException(
                      s"Server 'put response' didn't pass verifying for state=$this, serverMRoot=$serverMRoot"
                    )
                  )
                case Some(newMRoot) ⇒ // all is fine, send Confirm to server
                  for {
                    // safe new merkle root on the client
                    _ ← newMerkleRoot.flatMap(_.put(newMRoot))
                    // sign version with new merkle root and send back to node
                    signedState ← signNewState(version, newMRoot.asByteVector)
                  } yield signedState
              }
            case None ⇒
              Task.raiseError(
                new IllegalStateException(
                  s"Client put details isn't defined, it's should be defined at previous step"
                )
              )
          }
        }
      } yield signedNewState

    }

    // case when server confirmed changes persisted
    override def changesStored(): Task[Unit] = {
      // change global client state with new merkle root
      newMerkleRoot.flatMap(_.take).flatMap { newMRoot ⇒
        logger.debug(s"changesStored starts for key=$key, newMRoot=$newMRoot")

        clientStateMVar.flatMap(_.put(ClientState(newMRoot)))
      }
    }

    override def recoverState(): Task[Unit] = {
      logger.debug(s"Recover client state for put; mRoot=$merkleRoot")
      clientStateMVar.flatMap(_.put(ClientState(merkleRoot)))
    }

    /** Signs version concatenated with new merkle root */
    private def signNewState(ver: Long, mRoot: ByteVector): Task[ByteVector] = {
      signer.sign[Task](ByteVector.fromLong(ver) ++ mRoot).value.flatMap {
        case Left(err) ⇒ Task.raiseError(err)
        case Right(signetState) ⇒ Task(signetState.sign)
      }
    }

  }

  /**
   * Returns callbacks for finding ''value'' for specified ''key'' in remote MerkleBTree.
   *
   * @param key Plain text key
   */
  override def initGet(key: K): Task[SearchState[Task]] = {
    logger.debug(s"initGet starts for key=$key")

    for {
      csMVar ← clientStateMVar
      clientState ← csMVar.take
    } yield SearchStateImpl(key, clientState.merkleRoot.copy)

  }

  /**
   * Returns callbacks for finding start of range stream in remote MerkleBTree.
   *
   * @param from Plain text key, start of range
   */
  override def initRange(from: K): Task[SearchState[Task]] = {
    logger.debug(s"initRange starts for from=$from")

    for {
      csMVar ← clientStateMVar
      clientState ← csMVar.take
    } yield SearchStateImpl(from, clientState.merkleRoot.copy)

  }

  /**
   * Returns callbacks for saving encrypted ''key'' and ''value'' into remote MerkleBTree.
   *
   * @param key             Plain text key
   * @param valueChecksum  Checksum of encrypted value to be store
   * @param version         Dataset version expected to the client
   */
  override def initPut(key: K, valueChecksum: Hash, version: Long): Task[PutState[Task]] = {
    logger.debug(s"initPut starts put for key=$key, value=$valueChecksum, version=$version")

    for {
      csMVar ← clientStateMVar
      clientState ← csMVar.take
    } yield PutStateImpl(key, valueChecksum, version, clientState.merkleRoot.copy)

  }

  /**
   * Returns callbacks for deleting ''key value pair'' into remote MerkleBTree by specifying plain text key.
   *
   * @param key Plain text key
   * @param version Dataset version expected to the client
   */
  override def initRemove(key: K, version: Long): Task[RemoveState[Task]] = ???

  private def binarySearch(key: K, keys: Array[Key]): Task[SearchResult] = {
    import fluence.crypto.cipher.CryptoSearching._
    implicit val decrypt: Key ⇒ Task[K] = keyCrypt.decrypt
    search[Task, Key](keys).binarySearch(key)
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
    mRoot: Hash,
    mPath: MerklePath,
    keys: Array[Key],
    childsChecksums: Array[Hash]
  ): Task[(MerklePath, Int)] = {

    val nodeProof = verifier.getBranchProof(keys, childsChecksums, -1)

    if (verifier.checkProof(nodeProof, mRoot, mPath)) {
      binarySearch(key, keys).map { searchResult ⇒
        val updatedMPath = mPath.add(nodeProof.copy(substitutionIdx = searchResult.insertionPoint))

        updatedMPath → searchResult.insertionPoint
      }
    } else {
      Task.raiseError(
        new IllegalStateException(
          s"Checksum of branch didn't pass verifying for key=$key, Branch(${keys.mkString(",")}, ${keys.mkString(",")})"
        )
      )
    }
  }

}

object MerkleBTreeClient {

  /**
   * Creates base implementation of [[MerkleBTreeClientApi]] to calls for a remote MerkleBTree.
   *
   * @param initClientState General state holder for btree client. For new dataset should be ''None''
   * @param keyCrypt         Encrypting/decrypting provider for ''key''
   * @param cryptoHasher    Hash provider
   * @param signer          Algorithm to produce signatures. Used for sealing execState by contract owner
   * @param ord              The ordering to be used to compare keys.
   *
   * @tparam K The type of keys
   */
  def apply[K](
    initClientState: Option[ClientState],
    keyCrypt: Crypt[Task, K, Array[Byte]],
    cryptoHasher: CryptoHasher[Array[Byte], Hash],
    signer: Signer
  )(implicit ord: Ordering[K]): MerkleBTreeClient[K] = {
    import Key._
    import fluence.codec.Codec.identityCodec

    new MerkleBTreeClient[K](
      initClientState.getOrElse(ClientState(Hash.empty)),
      Crypt.transform(keyCrypt),
      BTreeVerifier(cryptoHasher),
      signer
    )
  }

  /**
   * Global state of BTree client.
   */
  case class ClientState(merkleRoot: Hash)

  object ClientState {
    def apply(merkleRoot: ByteVector): ClientState = ClientState(Hash(merkleRoot.toArray))
  }

}
