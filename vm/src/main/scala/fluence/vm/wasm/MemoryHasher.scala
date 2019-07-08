/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.vm.wasm

import java.nio.{ByteBuffer, ByteOrder}
import java.security.MessageDigest

import asmble.compile.jvm.{MemoryBuffer, MemoryByteBuffer}
import cats.{Applicative, Monad}
import cats.syntax.either._
import cats.syntax.apply._
import cats.data.EitherT
import fluence.crypto.{Crypto, CryptoError}
import fluence.crypto.Crypto.Hasher
import fluence.log.Log
import fluence.merkle.{BinaryMerkleTree, TrackingMemoryBuffer, TreeHasher}
import fluence.vm.VmError.{InternalVmError, VmMemoryError}
import fluence.vm.VmError.WasmVmError.GetVmStateError
import fluence.vm.utils._

import scala.language.higherKinds
import scala.util.Try

trait MemoryHasher {
  def computeMemoryHash[F[_]: Monad](): EitherT[F, GetVmStateError, Array[Byte]]
}

object MemoryHasher {

  val SHA_256 = "SHA-256"

  type Builder[F[_]] = MemoryBuffer => EitherT[F, GetVmStateError, MemoryHasher]

  /**
   * Builds memory hasher based on Merkle Tree with SHA-256 hash algorithm.
   *
   */
  def buildMerkleTreeHasher(memory: TrackingMemoryBuffer): Either[GetVmStateError, MemoryHasher] = {
    val digester = MessageDigest.getInstance(SHA_256)
    merkleHasher(memory, TreeHasher(digester))
  }

  /**
   * Instantiates the class, that get all memory and hash it with SHA-256 algorithm.
   *
   */
  def buildPlainHasher(memory: MemoryBuffer): MemoryHasher = {
    val leafDigester = MessageDigest.getInstance(SHA_256)
    val hasher: Hasher[ByteBuffer, Array[Byte]] = Crypto.liftFuncEither(
      bytes ⇒
        Try {
          leafDigester.reset()
          leafDigester.update(bytes)
          leafDigester.digest()
        }.toEither.left
          .map(err ⇒ CryptoError(s"Error on calculating $SHA_256 for plain memory hasher", Some(err)))
    )
    plainMemoryHasher(memory, hasher)
  }

  /**
   * Instantiates default hasher for different types of MemoryBuffer.
   *
   */
  def apply[F[_]: Monad: Log]: Builder[F] = {
    case m: TrackingMemoryBuffer =>
      Log.eitherT[F, GetVmStateError].info("TrackingMemoryBuffer with MerkleTree hasher will be used.") *>
        EitherT.fromEither[F](buildMerkleTreeHasher(m))
    case m =>
      Log.eitherT[F, GetVmStateError].info("Plain hasher will be used.") *>
        EitherT.rightT(buildPlainHasher(m))
  }

  def plainHasherBuilder[F[_]: Applicative](hasher: Hasher[ByteBuffer, Array[Byte]]): Builder[F] =
    m => EitherT.rightT[F, GetVmStateError](plainMemoryHasher(m, hasher))

  /**
   * Instantiates the class, that get all memory and hash it with `hasher` function on call.
   *
   * @param hasher hash function
   */
  def plainMemoryHasher(memory: MemoryBuffer, hasher: Hasher[ByteBuffer, Array[Byte]]): MemoryHasher = {
    new MemoryHasher {
      override def computeMemoryHash[F[_]: Monad](): EitherT[F, GetVmStateError, Array[Byte]] = {
        for {
          // could be overflow
          memoryArray <- safelyRunThrowable(
            {
              memory match {
                case m: TrackingMemoryBuffer =>
                  m.duplicate().clear()
                  m.bb
                case m: MemoryByteBuffer =>
                  m.duplicate().clear()
                  m.getBb
                case m =>
                  val arr = new Array[Byte](memory.capacity())
                  memory.duplicate().order(ByteOrder.LITTLE_ENDIAN).clear().get(arr)
                  ByteBuffer.wrap(arr)
              }

            },
            e =>
              VmMemoryError(
                s"Cannot copy memory with capacity ${memory.capacity()} to an array",
                Some(e)
            )
          )
          fullMemoryHash ← hasher(memoryArray).leftMap { e ⇒
            InternalVmError(s"Computing wasm memory hash failed", Some(e)): GetVmStateError
          }
        } yield fullMemoryHash
      }
    }
  }

  /**
   * Builds memory hasher based on Merkle Tree.
   *
   * @param memoryBuffer an access to memory
   * @param treeHasher class with hash functions
   * @return
   */
  def merkleHasher(
    memoryBuffer: TrackingMemoryBuffer,
    treeHasher: TreeHasher
  ): Either[GetVmStateError, MemoryHasher] = {
    for {
      tree <- Try(
        BinaryMerkleTree(
          treeHasher,
          memoryBuffer
        )
      ).toEither.leftMap(e => InternalVmError(s"Cannot create binary Merkle Tree", Some(e)): GetVmStateError)
    } yield {
      new MemoryHasher {
        override def computeMemoryHash[F[_]: Monad](): EitherT[F, GetVmStateError, Array[Byte]] = {
          safelyRunThrowable(
            tree.recalculateHash(),
            e => InternalVmError(s"Computing wasm memory hash failed", Some(e)): GetVmStateError
          )
        }
      }
    }
  }
}
