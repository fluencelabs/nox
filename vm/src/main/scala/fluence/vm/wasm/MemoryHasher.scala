package fluence.vm.wasm

import java.nio.ByteOrder

import asmble.compile.jvm.MemoryBuffer
import cats.{Applicative, Monad}
import cats.syntax.either._
import cats.data.EitherT
import fluence.crypto.Crypto.Hasher
import fluence.merkle.{BinaryMerkleTree, TrackingMemoryBuffer}
import fluence.vm.VmError.{InternalVmError, VmMemoryError}
import fluence.vm.VmError.WasmVmError.GetVmStateError
import fluence.vm.utils._

import scala.language.higherKinds
import scala.util.Try

trait MemoryHasher {
  def computeMemoryHash[F[_]: Monad](): EitherT[F, GetVmStateError, Array[Byte]]
}

object MemoryHasher {

  def apply(memory: MemoryBuffer, hasher: Hasher[Array[Byte], Array[Byte]]): Either[GetVmStateError, MemoryHasher] = {
    memory match {
      case m: TrackingMemoryBuffer =>
        merkleHasher(m, hasher)
      case m => Either.right(plainHasher(m, hasher))
    }
  }

  def plainHasher(memory: MemoryBuffer, hasher: Hasher[Array[Byte], Array[Byte]]): MemoryHasher = {
    new MemoryHasher {
      override def computeMemoryHash[F[_]: Monad](): EitherT[F, GetVmStateError, Array[Byte]] = {
        for {
          // could be overflow
          memoryArray <- safelyRunThrowable(
            {
              val arr = new Array[Byte](memory.capacity())
              memory.duplicate().order(ByteOrder.LITTLE_ENDIAN).clear().position(0).get(arr)
              arr
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

  def merkleHasher(
    memoryBuffer: TrackingMemoryBuffer,
    hasher: Hasher[Array[Byte], Array[Byte]]
  ): Either[GetVmStateError, MemoryHasher] = {
    for {
      tree <- Try(
        BinaryMerkleTree(memoryBuffer.chunkSize, hasher.unsafe, memoryBuffer)
      ).toEither.leftMap(e => InternalVmError(s"Cannot create binary Merkle Tree", Some(e)): GetVmStateError)
    } yield {
      new MemoryHasher {
        override def computeMemoryHash[FF[_]: Monad](): EitherT[FF, GetVmStateError, Array[Byte]] = {
          safelyRunThrowable(
            tree.recalculateHash(),
            e => InternalVmError(s"Computing wasm memory hash failed", Some(e)): GetVmStateError
          )
        }
      }
    }
  }
}
