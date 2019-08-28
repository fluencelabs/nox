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

import java.nio.ByteBuffer

import cats.Traverse.ops._
import cats.data.EitherT
import cats.instances.either._
import cats.instances.list._
import cats.syntax.either._
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{Applicative, Monad}
import com.softwaremill.sttp.Uri.QueryFragment.KeyValue
import com.softwaremill.sttp.circe.asJson
import com.softwaremill.sttp.{Multipart, SttpBackend, Uri, asStream, _}
import fluence.effects.castore.StoreError
import fluence.log.Log
import fs2.RaiseThrowable
import io.circe.{Decoder, DecodingFailure}
import scodec.bits.ByteVector

import scala.collection.immutable
import scala.concurrent.duration._
import scala.language.higherKinds

// TODO move somewhere else
object ResponseOps {
  import cats.data.EitherT
  import com.softwaremill.sttp.Response

  implicit class RichResponse[F[_], T, EE <: Throwable](resp: EitherT[F, Throwable, Response[T]])(
    implicit F: Monad[F]
  ) {
    val toEitherT: EitherT[F, String, T] = resp.leftMap(_.getMessage).subflatMap(_.body)
    def toEitherT[E](errFunc: String => E): EitherT[F, E, T] = toEitherT.leftMap(errFunc)
  }
}

class IpfsClient[F[_]: Monad](ipfsUri: Uri, readTimeout: FiniteDuration = 5.seconds)(
  implicit sttpBackend: SttpBackend[EitherT[F, Throwable, ?], fs2.Stream[F, ByteBuffer]]
) extends IpfsUploader[F] {

  import IpfsClient._
  import IpfsLsResponse._
  import ResponseOps._

  object Multihash {
    // https://github.com/multiformats/multicodec/blob/master/table.csv
    val SHA256 = ByteVector(0x12, 32) // 0x12 => SHA256; 32 = 256 bits in bytes
  }

  private val sttp = com.softwaremill.sttp.sttp.readTimeout(readTimeout)

  // URI for downloading data
  private val CatUri = ipfsUri.path("/api/v0/cat")

  // URI for listing data if it has nested resources
  private val LsUri = ipfsUri.path("/api/v0/ls")

  private val UploadUri = ipfsUri.path("/api/v0/add")

  // Converts 256-bits hash to a base58 IPFS address, prepending multihash bytes
  private def toAddress(hash: ByteVector): String = (Multihash.SHA256 ++ hash).toBase58

  // Converts base58 IPFS address to a 256-bits hash
  private def fromAddress(str: String): Either[String, ByteVector] =
    ByteVector.fromBase58Descriptive(str).map(_.drop(2))

  private def lsCall(hash: ByteVector)(implicit log: Log[F]): EitherT[F, StoreError, IpfsLsResponse] = {
    val address = toAddress(hash)
    val uri = LsUri.param("arg", address)
    for {
      _ <- Log.eitherT[F, StoreError].debug(s"IPFS `ls` started $uri")
      response <- sttp
        .response(asJson[IpfsLsResponse])
        .get(uri)
        .send()
        .leftMap(_.getMessage)
        .subflatMap(_.body)
        .leftMap(er => IpfsError(s"IPFS 'ls' error $uri: $er"): StoreError)
        .flatMapF {
          case Left(er) =>
            Log[F]
              .error(s"IPFS 'ls' deserialization error: $er")
              .as(
                (IpfsError(s"IPFS 'ls' deserialization error $uri", Some(er.error)): StoreError).asLeft[IpfsLsResponse]
              )
          case Right(r) ⇒
            r.asRight[StoreError].pure[F]
        }
        .flatTap { _ =>
          Log.eitherT[F, StoreError].debug(s"IPFS 'ls' finished $uri")
        }
    } yield response
  }

  /**
   * Generates URI for uploading to IPFS.
   *
   * @param onlyHash If true, IPFS will calculates the hash, without saving a data to IPFS
   * @param canBeMultiple If true, IPFS will wrap the list of files with directory and return a hash of this directory
   * @return
   */
  private def uploadUri(onlyHash: Boolean, canBeMultiple: Boolean) = {
    val multipleStr = canBeMultiple.toString
    UploadUri
      .queryFragment(KeyValue("pin", "true"))
      .queryFragment(KeyValue("path", ""))
      .queryFragment(KeyValue("only-hash", onlyHash.toString))
      .queryFragment(KeyValue("recursive", multipleStr))
      .queryFragment(KeyValue("wrap-with-directory", multipleStr))
  }

  /**
   * `add` operation. Wraps files with a directory if there are multiple files.
   *
   * @param data uploads to IPFS
   * @param onlyHash If true, only calculates the hash, without saving a data to IPFS
   */
  private def add[A: IpfsData](
    data: A,
    onlyHash: Boolean
  )(implicit log: Log[F]): EitherT[F, StoreError, ByteVector] = {
    val uri = uploadUri(onlyHash, IpfsData[A].wrapInDirectory)
    for {
      _ <- Log.eitherT[F, StoreError].debug(s"IPFS 'add' started $uri")
      multiparts <- IpfsData[A].toMultipart[F](data)
      responses <- addCall(uri, multiparts)
      _ <- assert[F](responses.nonEmpty, "IPFS 'add': Empty response")
      hash <- getParentHash(responses)
    } yield hash
  }

  /**
   * HTTP call to add multiparts to IPFS.
   *
   */
  private def addCall(uri: Uri, multiparts: immutable.Seq[Multipart])(
    implicit log: Log[F]
  ): EitherT[F, StoreError, List[UploadResponse]] =
    // raw response: {upload-response-object}\n{upload-response-object}...
    sttp
      .response(asListJson[UploadResponse])
      .post(uri)
      .multipartBody(multiparts)
      .send()
      .toEitherT { er =>
        val errorMessage = s"IPFS 'add' error $uri: $er"
        IpfsError(errorMessage): StoreError
      }
      .flatMapF {
        case Left(er) =>
          Log[F].error(s"IPFS 'add' deserialization error: $er") as
            (IpfsError(s"IPFS 'add' deserialization error $uri", Some(er)): StoreError).asLeft[List[UploadResponse]]

        case Right(r) ⇒
          r.asRight[StoreError].pure[F]
      }
      .flatTap { _ =>
        Log.eitherT[F, StoreError].debug(s"IPFS 'add' finished $uri")
      }

  /**
   * Returns hash of element with empty name. It is a wrapping directory's name.
   * If only one file was uploaded, a list has one element and a hash of this element will be returned.
   *
   * @param responses list of JSON responses from IPFS
   */
  private def getParentHash(responses: List[UploadResponse])(implicit log: Log[F]): EitherT[F, StoreError, ByteVector] =
    EitherT.fromEither[F] { // TODO recover the log, make normal EitherT
      for {
        namesWithHashes <- responses
          .map(
            r =>
              fromAddress(r.Hash).map(h => r.Name -> h).leftMap { e =>
                //logger.debug(s"IPFS 'add' hash ${r.Hash} is not correct")
                IpfsError(e)
              }
          )
          .sequence
        hash <- if (namesWithHashes.length == 1) Right(namesWithHashes.head._2)
        else {
          // if there is more then one JSON objects
          // find an object with an empty name - it will be an object with a directory
          namesWithHashes
            .find(_._1.isEmpty)
            .map(_._2)
            .toRight(
              IpfsError(
                s"IPFS 'add' error: incorrect response, expected at least 1 response with empty name, found 0. " +
                  s"Check 'wrap-with-directory' query flag in URI"
              ): StoreError
            )
        }
      } yield hash
    }

  /**
   * Returns hash of files from directory.
   * If hash belongs to file, returns the same hash.
   *
   * @param hash Content's hash
   */
  def ls(hash: ByteVector)(implicit log: Log[F]): EitherT[F, StoreError, List[ByteVector]] =
    for {
      rawResponse <- lsCall(hash)
      _ <- assert[F](
        rawResponse.Objects.size == 1,
        s"Expected a single object, got ${rawResponse.Objects.size}. Response: $rawResponse"
      )
      rawHashes = {
        val headObject = rawResponse.Objects.head
        if (headObject.Links.forall(_.Name.isEmpty)) List(headObject.Hash)
        else headObject.Links.map(_.Hash)
      }
      hashes <- rawHashes.map { h =>
        EitherT
          .fromEither[F](fromAddress(h))
          .leftMap(err => IpfsError(s"Cannot parse '$h' hex: $err"): StoreError)
      }.sequence
      _ ← Log.eitherT[F, StoreError].debug(s"IPFS 'ls' hashes: ${hashes.mkString(" ")}")
    } yield hashes

  /**
   * Downloads data from IPFS.
   *
   * @param hash data address in IPFS
   * @return
   */
  def download(hash: ByteVector)(implicit log: Log[F]): EitherT[F, StoreError, fs2.Stream[F, ByteBuffer]] = {
    val address = toAddress(hash)
    val uri = CatUri.param("arg", address)
    for {
      _ <- Log.eitherT[F, StoreError].debug(s"IPFS 'download' started $uri")
      response <- sttp
        .response(asStream[fs2.Stream[F, ByteBuffer]])
        .get(uri)
        .send()
        .toEitherT { er =>
          val errorMessage = s"IPFS 'download' error $uri: $er"
          IpfsError(errorMessage): StoreError
        }
        .flatTap { _ =>
          Log.eitherT[F, StoreError].debug(s"IPFS 'download' finished $uri")
        }
    } yield response
  }

  /**
   * Only calculates hash - no data will be persisted on IPFS.
   *
   * @return hash of data
   */
  def calculateHash[A: IpfsData](data: A)(implicit log: Log[F]): EitherT[F, StoreError, ByteVector] =
    add(data, onlyHash = true)

  /**
   * Uploads data to IPFS
   *
   * @return hash of data
   */
  def upload[A: IpfsData](data: A)(implicit log: Log[F]): EitherT[F, StoreError, ByteVector] =
    add(data, onlyHash = false)
}

object IpfsClient {
  import io.circe.fs2.stringStreamParser

  private[ipfs] def assert[F[_]: Applicative](test: Boolean, errorMessage: String): EitherT[F, IpfsError, Unit] =
    EitherT.fromEither[F](Either.cond(test, (), IpfsError(errorMessage)))

  // parses application/json+stream like {object1}\n{object2}...
  private[ipfs] def asListJson[B: Decoder: IsOption]: ResponseAs[Decoder.Result[List[B]], Nothing] = {
    implicit val rt = new RaiseThrowable[fs2.Pure] {}
    asString
      .map(fs2.Stream.emit)
      .map(
        _.through(stringStreamParser[fs2.Pure]).attempt
          .map(_.leftMap {
            case e: DecodingFailure => e
            case e: Throwable       => DecodingFailure(e.getLocalizedMessage, Nil)
          })
          .toList
          .map(_.map(_.as[B]).flatMap(identity))
          .sequence
      )
  }
}
