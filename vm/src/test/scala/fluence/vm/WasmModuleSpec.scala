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
import java.lang.reflect.{InvocationTargetException, Method}
import java.nio.ByteBuffer

import asmble.run.jvm.Module.Compiled
import asmble.run.jvm.ScriptContext
import cats.data.EitherT
import fluence.crypto.CryptoError
import fluence.vm.VmError.{InitializationError, InternalVmError}
import fluence.vm.wasm.WasmModule
import org.mockito.Mockito
import org.scalatest.mockito.MockitoSugar
import org.scalatest.{Matchers, WordSpec}

class WasmModuleSpec extends WordSpec with Matchers with MockitoSugar {

  "apply" should {
    "returns an error" when {
      "unable to initialize module" in {
        val scriptCtx = mock[ScriptContext]
        val module = mock[Compiled]
        Mockito.when(module.getName).thenReturn("test-module-name")
        Mockito.when(module.instance(scriptCtx)).thenThrow(new RuntimeException("boom!"))

        WasmModule(module, scriptCtx, "", "", "") match {
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

        WasmModule(module, scriptCtx, "", "", "") match {
          case Right(_) ⇒
            fail("Should be error appeared")
          case Left(e) ⇒
            e.getMessage shouldBe "Unable to getting memory from module=test-module-name"
            e.getCause shouldBe a[InvocationTargetException]
            e shouldBe a[InitializationError]
        }
      }
    }

    "return module instance" when {
      "there is no module memory" in {
        val instance = new {}
        val module = mock[Compiled]
        Mockito.when(module.getName).thenReturn("test-module-name")
        val scriptCtx = mock[ScriptContext]
        Mockito.when(module.instance(scriptCtx)).thenReturn(instance, null)

        WasmModule(module, scriptCtx, "", "", "") match {
          case Right(moduleInstance) ⇒
            moduleInstance.readMemory(0, 0) shouldBe None
          case Left(e) ⇒
            fail("Error shouldn't appears.")
        }
      }

      "module has a memory" in {
        val instance = new { def getMemory: ByteBuffer = ByteBuffer.wrap(Array[Byte](1, 2, 3)) }
        val module = mock[Compiled]
        Mockito.when(module.getName).thenReturn("test-module-name")
        val scriptCtx = mock[ScriptContext]
        Mockito.when(module.instance(scriptCtx)).thenReturn(instance, null)

        WasmModule(module, scriptCtx, "", "", "") match {
          case Right(moduleInstance) ⇒
            for {
              memoryRegion <- moduleInstance.readMemory(0, 100)
            } yield memoryRegion should contain allOf (1, 2, 3)
          case Left(e) ⇒
            fail("Error shouldn't appears.")
        }
      }
    }
  }

  "innerState" should {
    "returns an error" when {
      "hasher returns an error" in {
        val instance = new { def getMemory: ByteBuffer = ByteBuffer.wrap(Array[Byte](1, 2, 3)) }
        val result =
          createInstance(instance).computeHash(arr ⇒ EitherT.leftT(CryptoError("error!"))).value.left.get

        result.message shouldBe "Getting internal state for module=test-module-name failed"
        result.getCause shouldBe a[CryptoError]
        result shouldBe a[InternalVmError]
      }

      "working with memory is failed" in {
        val memoryBuffer = ByteBuffer.wrap(Array[Byte](1, 2, 3))
        memoryBuffer.position(1)
        val instance = new { def getMemory: ByteBuffer = null }
        val result = createInstance(instance).computeHash(arr ⇒ EitherT.rightT(arr)).value.left.get

        result.message shouldBe "Presenting memory as an array for module=test-module-name failed"
        result shouldBe a[InternalVmError]
      }
    }

    "returns empty array of bytes" when {
      "memory isn't present in a module" in {
        val result = createInstance(new {}).computeHash(arr ⇒ EitherT.rightT(arr)).value.right.get
        result shouldBe Array.emptyByteArray
      }
    }

    "returns hash of VM's state" when {
      "memory is present in a module" in {
        val instance = new { def getMemory: ByteBuffer = ByteBuffer.wrap(Array[Byte](1, 2, 3)) }
        val result = createInstance(instance).computeHash(arr ⇒ EitherT.rightT(arr)).value.right.get
        result should contain allOf (1, 2, 3)
      }
    }

    "getting inner state should be idempotent for VM inner state" in {
      // create a memory, set ByteBuffer position to 1, after getting state buffer
      // should be completely the same that was before.
      val memoryBuffer = ByteBuffer.wrap(Array[Byte](1, 2, 3))
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
      val instance = new { def getMemory: ByteBuffer = memoryBuffer }
      val result = createInstance(instance).computeHash(arr ⇒ EitherT.rightT(arr)).value.right.get
      // checks that result is correct
      result should contain allOf (1, 2, 3)
      // checks that 'memoryBuffer' wasn't change
      memoryBuffer shouldBe expected

    }

  }

  private def createInstance(instance: AnyRef): WasmModule = {
    val module = mock[Compiled]
    Mockito.when(module.getName).thenReturn("test-module-name")
    val scriptCtx = mock[ScriptContext]
    Mockito.when(module.instance(scriptCtx)).thenReturn(instance, null)

    WasmModule(module, scriptCtx, "", "", "").right.get
  }

}
