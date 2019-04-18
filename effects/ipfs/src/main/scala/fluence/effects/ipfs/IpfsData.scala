package fluence.effects.ipfs

import java.nio.file.{Files, Path}

import cats.{Applicative, Monad}
import cats.data.EitherT
import com.softwaremill.sttp.Multipart
import com.softwaremill.sttp._
import fluence.effects.castore.StoreError
import scodec.bits.ByteVector

import scala.collection.immutable
import scala.language.higherKinds

trait IpfsData[F[_]] {
  def toMultipart: EitherT[F, IpfsError, immutable.Seq[Multipart]]
  def canBeMultiple: Boolean
}

object IpfsData {

  def apply[F[_]: Applicative](bytes: ByteVector): IpfsData[F] = new IpfsData[F] {
    override def toMultipart: EitherT[F, IpfsError, immutable.Seq[Multipart]] =
      EitherT.pure(immutable.Seq(multipart("", ByteArrayBody(bytes.toArray))))

    override def canBeMultiple: Boolean = false
  }

  def apply[F[_]: Monad](path: Path): IpfsData[F] = new IpfsData[F] {
    import IpfsClient._

    /**
     * Returns incoming path if it is a file, return a list of files, if the incoming path is a directory.
     * Validates if the directory doesn't have nested directories.
     */
    private def listPaths(path: Path): EitherT[F, IpfsError, immutable.Seq[Path]] = {
      import scala.collection.JavaConverters._
      if (Files.isDirectory(path)) {
        val allFiles = Files.list(path).iterator().asScala.to[immutable.Seq]
        val allFilesIsRegular = allFiles.forall(p => Files.isRegularFile(p))
        assert(
          allFilesIsRegular,
          s"IPFS 'listPaths' error: expected flat directory, found nested directories in ${path.getFileName}"
        ).map(_ => allFiles)
      } else EitherT.pure(immutable.Seq(path))
    }

    override def toMultipart: EitherT[F, IpfsError, immutable.Seq[Multipart]] =
      for {
        _ <- assert(Files.exists(path), s"IPFS 'add' error: file '${path.getFileName}' does not exist")
        pathsList <- listPaths(path)
        parts = pathsList.map(p => multipartFile("", p))
        _ <- assert(parts.nonEmpty, s"IPFS 'add' error: directory ${path.getFileName} is empty")
      } yield parts

    override def canBeMultiple: Boolean = true
  }
}
