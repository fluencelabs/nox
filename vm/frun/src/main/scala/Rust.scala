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

import Rust.RustDirectory
import cats.data.NonEmptyList
import cats.effect.IO

import scala.sys.process.ProcessLogger
import scala.util.control.NoStackTrace

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

object Rust {
  val RustDirectory = "/rust"

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

  private val NopLogger = new ProcessLogger {
    override def out(s: => String): Unit = {}
    override def err(s: => String): Unit = {}
    override def buffer[T](f: => T): T = f
  }

  private def exec(error: Throwable, cmds: String*): IO[Unit] = {
    import scala.sys.process._

    val processIO = cmds.toList match {
      case head :: tail => IO(tail.foldLeft(Process(head)) { case (pb, next) => pb #| next })
      case Nil => IO.raiseError(new RuntimeException("exec was called with empty argument list"))
    }

    for {
      process <- processIO
      code <- IO(process.run(NopLogger).exitValue())
      _ <- if (code != 0) IO.raiseError(error) else IO.unit
    } yield ()
  }

  private def compileProject(): IO[NonEmptyList[String]] =
    for {
      _ <- run("cargo +nightly build --lib --target wasm32-unknown-unknown --release")
      targetDir <- IO(Paths.get(RustDirectory).resolve("target/wasm32-unknown-unknown/release").toAbsolutePath.toString)
      wasmFiles <- Utils.getWasmFiles(targetDir)
    } yield wasmFiles

  private def checkRustInstalled(): IO[Unit] = {
    def check(component: String) = exec(NotFoundInPath(component), s"which $component")
    for {
      _ <- check("cargo")
      _ <- check("rustup")
      _ <- exec(Wasm32NotFound, "rustup target list", "grep -q wasm32-unknown-unknown")
    } yield ()
  }

  def compile(): IO[NonEmptyList[String]] =
    for {
      _ <- checkRustInstalled()
      _ <- checkProjectExists()
      wasmFiles <- compileProject()
    } yield wasmFiles
}
