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

package fluence.effects.ipfs

import java.nio.file.{Files, Path}

import cats.Monad
import cats.data.EitherT
import com.softwaremill.sttp.{Multipart, _}
import fluence.effects.ipfs.IpfsClient.assert
import scodec.bits.ByteVector

import scala.collection.immutable
import scala.language.higherKinds

trait IpfsData[A] {
  def toMultipart[F[_]: Monad](data: A): EitherT[F, IpfsError, immutable.Seq[Multipart]]
  val canBeMultiple: Boolean
}

object IpfsData {

  implicit val chunksData: IpfsData[List[ByteVector]] = new IpfsData[List[ByteVector]] {

    def toMultipart[F[_]: Monad](data: List[ByteVector]): EitherT[F, IpfsError, immutable.Seq[Multipart]] = {
      EitherT.pure(data.zipWithIndex.map {
        case (chunk, idx) =>
          multipart(idx.toString, ByteArrayBody(chunk.toArray))
      })
    }

    override val canBeMultiple: Boolean = true
  }

  implicit val bytesData: IpfsData[ByteVector] = new IpfsData[ByteVector] {
    override def toMultipart[F[_]: Monad](data: ByteVector): EitherT[F, IpfsError, immutable.Seq[Multipart]] =
      EitherT.pure(immutable.Seq(multipart("", ByteArrayBody(data.toArray))))

    override val canBeMultiple: Boolean = false
  }

  /**
   * Uploads files to IPFS node. Supports only one file or files in one directory, without nested directories.
   */
  implicit val pathData: IpfsData[Path] = new IpfsData[Path] {

    /**
     * Returns incoming path if it is a file, return a list of files, if the incoming path is a directory.
     * Validates if the directory doesn't have nested directories.
     */
    private def listPaths[F[_]: Monad](path: Path): EitherT[F, IpfsError, immutable.Seq[Path]] = {
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

    override def toMultipart[F[_]: Monad](path: Path): EitherT[F, IpfsError, immutable.Seq[Multipart]] = {
      for {
        _ <- assert(Files.exists(path), s"IPFS 'add' error: file '${path.getFileName}' does not exist")
        pathsList <- listPaths(path)
        parts = pathsList.map(p => multipartFile("", p))
        _ <- assert(parts.nonEmpty, s"IPFS 'add' error: directory ${path.getFileName} is empty")
      } yield parts
    }

    override val canBeMultiple: Boolean = true
  }

  def apply[A](implicit id: IpfsData[A]): IpfsData[A] = id
}
