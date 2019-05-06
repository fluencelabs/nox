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

import java.nio.file.Paths

import cats.data.NonEmptyList
import cats.effect.IO

import scala.util.control.NoStackTrace

object Rust {
  private val RustDirectory = "/rust"

  object NoProjectFound extends NoStackTrace {
    override def getMessage: String =
      s"$RustDirectory/Cargo.toml not found\n" +
        s"Please pass Rust project as a volume: `-v /path/to/your/project:$RustDirectory`"
  }

  object CompilationFailed extends NoStackTrace {
    override def getMessage: String = s"Rust project compilation failed"
  }

  case class NotFoundInPath(component: String) extends NoStackTrace {
    override def getMessage: String = s"$component was not found in $$PATH (`command -v $component` exit code != 0)"
  }

  object Wasm32NotFound extends NoStackTrace {
    override def getMessage: String =
      "wasm32-unknown-unknown isn't found in Rust targets.\nfix: rustup target add wasm32-unknown-unknown --toolchain nightly"
  }

  private def checkProjectExists(): IO[Unit] =
    for {
      path <- IO(Paths.get(RustDirectory, "Cargo.toml"))
      exists <- IO(path.toFile.exists())
      _ <- if (!exists) IO.raiseError(NoProjectFound) else IO.unit
    } yield ()

  private def run(cmd: String): IO[Unit] = {
    import scala.sys.process._
    for {
      workDir <- IO(Paths.get(RustDirectory).toFile)
      code <- IO(Process(cmd, workDir).run().exitValue())
      _ <- if (code != 0) IO.raiseError(CompilationFailed) else IO.unit
    } yield ()
  }

  private def compileProject(): IO[NonEmptyList[String]] =
    for {
      _ <- run("cargo +nightly build --lib --target wasm32-unknown-unknown --release")
      targetDir <- IO(Paths.get(RustDirectory).resolve("target/wasm32-unknown-unknown/release").toAbsolutePath.toString)
      wasmFiles <- Utils.getWasmFiles(targetDir)
    } yield wasmFiles

  private def checkRustInstalled(): IO[Unit] = {
    def raise[AA](e: Throwable): Throwable => IO[AA] = _ => IO.raiseError(e)
    def runOrRaise(cmd: String, e: Throwable) = run(cmd).handleErrorWith(raise(e))
    def check(component: String) = runOrRaise(s"command -v $component", NotFoundInPath(component))
    for {
      _ <- check("cargo")
      _ <- check("rustup")
      _ <- runOrRaise("rustup target list | grep -q wasm32-unknown-unknown", Wasm32NotFound)
    } yield ()
  }

  def compile(): IO[NonEmptyList[String]] =
    for {
      _ <- checkRustInstalled()
      _ <- checkProjectExists()
      wasmFiles <- compileProject()
    } yield wasmFiles
}
