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

  def compile(): IO[NonEmptyList[String]] =
    for {
      _ <- checkProjectExists()
      wasmFiles <- compileProject()
    } yield wasmFiles
}
