package fluence.effects.ipfs

import java.nio.ByteBuffer

import cats.Monad
import cats.data.EitherT
import cats.instances.list._
import cats.Traverse.ops._
import com.softwaremill.sttp.Uri.QueryFragment.KeyValue
import com.softwaremill.sttp.{ByteArrayBody, Multipart, SttpBackend, Uri, asStream, sttp}
import fluence.effects.castore.StoreError
import scodec.bits.ByteVector
import com.softwaremill.sttp.circe.asJson

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

class IpfsClient[F[_]](ipfsUri: Uri)(
  implicit sttpBackend: SttpBackend[EitherT[F, Throwable, ?], fs2.Stream[F, ByteBuffer]],
  F: cats.Monad[F]
) extends slogging.LazyLogging {
  import IpfsLsResponse._
  import ResponseOps._

  object Multihash {
    // https://github.com/multiformats/multicodec/blob/master/table.csv
    val SHA256 = ByteVector(0x12, 32) // 0x12 => SHA256; 32 = 256 bits in bytes
  }

  // URI for downloading the file
  private val CatUri = ipfsUri.path("/api/v0/cat")

  private val LsUri = ipfsUri.path("/api/v0/ls")

  private val UploadUri = ipfsUri.path("/api/v0/add")

  // Converts 256-bits hash to an bas58 IPFS address, prepending multihash bytes
  private def toAddress(hash: ByteVector): String = (Multihash.SHA256 ++ hash).toBase58

  private def fromAddress(str: String) = ByteVector.fromBase58Descriptive(str).map(_.drop(2))

  def download(hash: ByteVector): EitherT[F, StoreError, fs2.Stream[F, ByteBuffer]] = {
    val address = toAddress(hash)
    val uri = CatUri.param("arg", address)
    for {
      _ <- EitherT.pure[F, StoreError](logger.debug(s"IPFS download started $uri"))
      response <- sttp
        .response(asStream[fs2.Stream[F, ByteBuffer]])
        .get(uri)
        .send()
        .toEitherT { er =>
          val errorMessage = s"IPFS download error $uri: $er"
          IpfsError(errorMessage)
        }
        .map { r =>
          logger.debug(s"IPFS download finished $uri")
          r
        }
        .leftMap(identity[StoreError])
    } yield response

  }

  private def lsRaw(hash: ByteVector): EitherT[F, StoreError, IpfsLsResponse] = {
    val address = toAddress(hash)
    val uri = LsUri.param("arg", address)
    for {
      _ <- EitherT.pure[F, StoreError](logger.debug(s"IPFS `ls` started $uri"))
      response <- sttp
        .response(asJson[IpfsLsResponse])
        .get(uri)
        .send()
        .toEitherT { er =>
          val errorMessage = s"IPFS 'ls' error $uri: $er"
          IpfsError(errorMessage)
        }
        .subflatMap(_.left.map { er =>
          logger.error(s"Deserialization error: $er")
          IpfsError(s"IPFS 'ls' deserialization error $uri.", Some(er.error))
        })
        .map { r =>
          logger.debug(s"IPFS 'ls' finished $uri")
          r
        }
        .leftMap(identity[StoreError])
    } yield response

  }

  private def assert(test: Boolean, error: IpfsError): EitherT[F, StoreError, Unit] = {
    EitherT.fromEither(Either.cond(test, (), error.asInstanceOf[StoreError]))
  }

  /**
   * Returns hash of files from directory.
   * If hash belongs to file, returns the same hash.
   *
   * @param hash Content's hash
   */
  def ls(hash: ByteVector): EitherT[F, StoreError, List[ByteVector]] =
    for {
      rawResponse <- lsRaw(hash)
      _ <- assert(
        rawResponse.Objects.size == 1,
        IpfsError(s"Expected a single object, got ${rawResponse.Objects.size}. Response: $rawResponse")
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
    } yield {
      logger.debug(s"IPFS 'ls' hashes: ${hashes.mkString(" ")}")
      hashes
    }

  private def add(data: ByteVector, onlyHash: Boolean): EitherT[F, StoreError, ByteVector] = {
    val uri = UploadUri
      .queryFragment(KeyValue("pin", "true"))
      .queryFragment(KeyValue("path", ""))
      .queryFragment(KeyValue("only-hash", onlyHash.toString))
    for {
      _ <- EitherT.pure[F, StoreError](logger.debug(s"IPFS upload started $uri"))
      response <- sttp
        .response(asJson[UploadResponse])
        .post(uri)
        .multipartBody(Multipart("", ByteArrayBody(data.toArray)))
        .send()
        .toEitherT { er =>
          val errorMessage = s"IPFS 'add' error $uri: $er"
          IpfsError(errorMessage)
        }
        .subflatMap(_.left.map { er =>
          logger.error(s"Deserialization error: $er")
          IpfsError(s"IPFS 'add' deserialization error $uri.", Some(er.error))
        })
        .map { r =>
          logger.debug(s"IPFS 'add' finished $uri")
          r
        }
        .leftMap(identity[StoreError])
      hash <- EitherT.fromEither[F](fromAddress(response.Hash))
        .leftMap { r =>
          logger.debug(s"IPFS 'add' hash ${response.Hash} is not correct $uri")
          IpfsError(r)
        }.leftMap(identity[StoreError])
    } yield hash
  }

  /**
   * Only calculate hash - do not write to disk.
   * @return hash of data
   */
  def calculateHash(data: ByteVector): EitherT[F, StoreError, ByteVector] = add(data, onlyHash = true)

  /**
   * Uploads bytes to IPFS node
   * @return hash of data
   */
  def upload(data: ByteVector): EitherT[F, StoreError, ByteVector] = add(data, onlyHash = false)
