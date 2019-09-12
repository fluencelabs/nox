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

package fluence.vm
import java.lang.reflect.InvocationTargetException
import java.nio.ByteBuffer

import asmble.compile.jvm.{MemoryBuffer, MemoryByteBuffer}
import asmble.run.jvm.Module.Compiled
import asmble.run.jvm.ScriptContext
import cats.{~>, Id, Monad}
import cats.data.EitherT
import cats.effect.{IO, Timer}
import fluence.crypto.Crypto.Hasher
import fluence.crypto.CryptoError
import fluence.log.{Log, LogFactory}
import fluence.vm.VmError.{InitializationError, InternalVmError}
import fluence.vm.wasm.MemoryHasher
import fluence.vm.wasm.module.{MainWasmModule, WasmModule}
import org.mockito.Mockito
import org.mockito.stubbing.OngoingStubbing
import org.scalatest.{Matchers, WordSpec}
import org.scalatestplus.mockito.MockitoSugar

import scala.concurrent.ExecutionContext
import scala.language.higherKinds

class WasmModuleSpec extends WordSpec with Matchers with MockitoSugar {

  implicit val ioTimer: Timer[IO] = IO.timer(ExecutionContext.global)
  implicit val log: Log[IO] = LogFactory.forPrintln[IO](Log.Error).init(getClass.getSimpleName).unsafeRunSync()
  implicit val logId: Log[Id] = log.mapK(new (IO ~> Id) {
    override def apply[A](fa: IO[A]): Id[A] = fa.unsafeRunSync()
  })

  "apply" should {
    "returns an error" when {
      "unable to initialize main module" in {
        val scriptCtx = mock[ScriptContext]
        val module = mock[Compiled]
        Mockito.when(module.getName).thenReturn("test-module-name")
        Mockito.when(module.instance(scriptCtx)).thenThrow(new RuntimeException("boom!"))
        addClsStubbing(module)

        MainWasmModule(module, scriptCtx, MemoryHasher.apply, "", "", "").value match {
          case Right(_) ⇒
            fail("Should be error appeared")
          case Left(e) ⇒
            e.getMessage shouldBe "Unable to initialize module=test-module-name"
            e.getCause shouldBe a[RuntimeException]
            e shouldBe a[InitializationError]
        }
      }

      "unable to initialize side module" in {
        val scriptCtx = mock[ScriptContext]
        val module = mock[Compiled]
        Mockito.when(module.getName).thenReturn("test-module-name")
        Mockito.when(module.instance(scriptCtx)).thenThrow(new RuntimeException("boom!"))
        addClsStubbing(module)

        WasmModule(module, scriptCtx, MemoryHasher.apply).value match {
          case Right(_) ⇒
            fail("Should be error appeared")
          case Left(e) ⇒
            e.getMessage shouldBe "Unable to initialize module=test-module-name"
            e.getCause shouldBe a[RuntimeException]
            e shouldBe a[InitializationError]
        }
      }

      "unable to getting memory" in {
        val instance = new { def getMemory: ByteBuffer = throw new RuntimeException("boom!") }
        val module = mock[Compiled]
        Mockito.when(module.getName).thenReturn("test-module-name")
        val scriptCtx = mock[ScriptContext]
        Mockito.when(module.instance(scriptCtx)).thenReturn(instance, null)
        addClsStubbing(module)

        WasmModule(module, scriptCtx, MemoryHasher.apply).value match {
          case Right(_) ⇒
            fail("Should be error appeared")
          case Left(e) ⇒
            e.getMessage shouldBe "Unable to get memory from module=test-module-name"
            e.getCause shouldBe a[InvocationTargetException]
            e shouldBe a[InitializationError]
        }
      }
    }
  }

  "innerState" should {
    "returns an error" when {
      "hasher returns an error" in {
        val instance = new { def getMemory: MemoryBuffer = new MemoryByteBuffer(ByteBuffer.wrap(Array[Byte](1, 2, 3))) }
        val badHasher: Hasher[ByteBuffer, Array[Byte]] = new Hasher[ByteBuffer, Array[Byte]] {
          override def apply[F[_]](input: ByteBuffer)(
            implicit evidence$2: Monad[F]
          ): EitherT[F, CryptoError, Array[Byte]] = EitherT.fromEither(Left(CryptoError("test error")))
        }
        val result =
          createWasmModuleFull(instance, MemoryHasher.plainHasherBuilder(badHasher)).computeStateHash().value.left.get

        result.getMessage shouldBe "Computing wasm memory hash failed"
        result.getCause shouldBe a[CryptoError]
        result shouldBe a[InternalVmError]
      }
    }

    "returns hash of VM's state" when {
      "memory is present in a module" in {
        val instance = new {
          def getMemory: MemoryByteBuffer = new MemoryByteBuffer(ByteBuffer.wrap(Array[Byte](1, 2, 3)))
        }
        val result = createWasmModulePlainHasher(instance).computeStateHash().value.right.get
        result should contain allOf (1, 2, 3)
      }
    }

    "getting inner state should be idempotent for VM inner state" in {
      // create a memory, set ByteBuffer position to 1, after getting state buffer
      // should be completely the same that was before.
      val memoryBuffer = new MemoryByteBuffer(ByteBuffer.wrap(Array[Byte](1, 2, 3)))
      memoryBuffer.position(1)
      val expected = memoryBuffer.duplicate()

      // checks that 'expected' is really the same as 'memoryBuffer'
      memoryBuffer shouldBe expected
      // checks that 'expected' don't change when 'memoryBuffer' changed
      memoryBuffer.position(2)
      memoryBuffer should not be expected
      // reverts prev step and check that 'memoryBuffer' is equal 'expected'
      memoryBuffer.position(1)
      memoryBuffer shouldBe expected
      // getting inner VM state
      val instance = new { def getMemory: MemoryByteBuffer = memoryBuffer }
      val result = createWasmModulePlainHasher(instance).computeStateHash().value.right.get
      // checks that result is correct
      result should contain allOf (1, 2, 3)
      // checks that 'memoryBuffer' wasn't change
      memoryBuffer shouldBe expected

    }

  }

  private def addClsStubbing(module: Compiled) = {
    val whenStubbing: OngoingStubbing[Class[_]] = Mockito.when(module.getCls)
    whenStubbing.thenReturn(module.getClass, null)
  }

  private def createWasmModulePlainHasher(instance: AnyRef): WasmModule = {
    val plainHasher = new Hasher[ByteBuffer, Array[Byte]] {
      override def apply[F[_]](input: ByteBuffer)(
        implicit evidence$2: Monad[F]
      ): EitherT[F, CryptoError, Array[Byte]] = {
        val arr = new Array[Byte](input.capacity())
        val dup = input.duplicate()
        dup.clear()
        dup.get(arr)
        EitherT.pure(arr)
      }
    }

    createWasmModuleFull(instance, (m: MemoryBuffer) => EitherT.rightT(MemoryHasher.plainMemoryHasher(m, plainHasher)))
  }
  private def createWasmModule(instance: AnyRef): WasmModule = createWasmModuleFull(instance, MemoryHasher.apply)

  private def createWasmModuleFull(instance: AnyRef, builder: MemoryHasher.Builder[Id]): WasmModule = {
    val module = mock[Compiled]
    Mockito.when(module.getName).thenReturn("test-module-name")
    val scriptCtx = mock[ScriptContext]
    Mockito.when(module.instance(scriptCtx)).thenReturn(instance, null)

    addClsStubbing(module)

    WasmModule(module, scriptCtx, builder).value.right.get
  }

}

class TestClass {
  val getMemory: MemoryBuffer = new MemoryByteBuffer(ByteBuffer.wrap(Array[Byte](1, 2, 3)))
}
