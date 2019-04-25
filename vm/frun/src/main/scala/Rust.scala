import java.nio.file.{Path, Paths}

import cats.effect.IO

import scala.util.control.NoStackTrace

object Rust {
  val RustDirectory = "/rust"
  object NoProjectFound extends NoStackTrace {
    override def getMessage: String =
      s"$RustDirectory/Cargo.toml not found\n" +
      s"Please pass Rust project as a volume: `-v /path/to/your/project:$RustDirectory`"
  }
  object CompilationFailed extends NoStackTrace {
    override def getMessage: String = s"Rust project compilation failed"
  }

  def checkProjectExists(): IO[Unit] =  for {
    path <- IO(Paths.get(RustDirectory, "Cargo.toml"))
    exists <- IO(path.toFile.exists())
    _ <- if (!exists) IO.raiseError(NoProjectFound) else IO.unit
  } yield ()

  def run(cmd: String): IO[Unit] = {
    import scala.sys.process._
    for {
      code <- IO(cmd.run().exitValue())
      _ <- if (code != 0) IO.raiseError(CompilationFailed) else IO.unit
    } yield ()
  }

  def compileProject(): IO[Path] = for {
    _ <- run("")
  } yield path

  def compile(): IO[Unit] = for {
    _ <- checkProjectExists()
  } yield ()
}
