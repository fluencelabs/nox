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

package fluence.effects.swarm

import java.nio.ByteBuffer

import cats.Monad
import cats.data.EitherT
import cats.effect.IO
import cats.syntax.apply._
import cats.syntax.applicative._
import cats.syntax.functor._
import cats.syntax.either._
import cats.syntax.flatMap._
import com.softwaremill.sttp.{Uri, asStream, _}
import com.softwaremill.sttp.circe._
import fluence.crypto.Crypto.Hasher
import fluence.effects.sttp.SttpStreamEffect
import fluence.effects.sttp.syntax._
import fluence.effects.swarm.crypto.Keccak256Hasher
import fluence.effects.swarm.crypto.Secp256k1Signer.Signer
import fluence.effects.swarm.requests._
import fluence.effects.swarm.responses.Manifest
import fluence.log.Log
import io.circe.syntax._
import io.circe.{Json, Printer}
import scodec.bits.ByteVector

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

// TODO implement extended swarm functions https://github.com/fluencelabs/dataengine/issues/52
// TODO split errors from Swarm and internal errors
// TODO add logs
/**
 * Client for working with Swarm.
 *
 * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#
 * @param swarmUri HTTP address of trusted swarm node
 * @param hasher hashing algorithm. Must be Keccak SHA-3 algorithm for real Swarm node or another for test purposes
 *               @see https://en.wikipedia.org/wiki/SHA-3
 */
class SwarmClient[F[_]: Monad: SttpStreamEffect](swarmUri: Uri, readTimeout: FiniteDuration)(
  implicit hasher: Hasher[ByteVector, ByteVector]
) {

  import BzzProtocol._
  import fluence.effects.swarm.helpers.ResponseOps._

  // unpretty printer for http requests
  private val printer = Printer.noSpaces.copy(dropNullValues = true)

  private val sttp = com.softwaremill.sttp.sttp.readTimeout(readTimeout)

  // generate body from json for http requests
  private def jsonToBytes(json: Json) = printer.pretty(json).getBytes

  /**
   * Generate uri for requests.
   *
   * @param bzzProtocol protocol for requests, e.g. `bzz:/`, `bzz-resource:/`, etc
   * @param target hash of resource (file, metadata, manifest) or address from ENS.
   * @param path additional parameters for request
   * @return generated uri
   */
  private def uri(bzzProtocol: BzzProtocol, target: String, path: Seq[String] = Nil) =
    swarmUri.path(Seq(bzzProtocol.protocol, target) ++ path)

  private def uri(bzzUri: BzzProtocol) = swarmUri.path(bzzUri.protocol)

  /**
   * Downloads a file from Swarm
   *
   * @see https://swarm-guide.readthedocs.io/en/latest/up-and-download.html
   * @param hash Hash of resource (file, metadata, manifest) or address from ENS.
   *
   */
  def download(hash: String)(implicit log: Log[F]): EitherT[F, SwarmError, Array[Byte]] = {
    val downloadURI = uri(Bzz, hash)
    Log.eitherT[F, SwarmError].debug(s"Swarm download started $downloadURI") *>
      sttp
        .response(asByteArray)
        .get(downloadURI)
        .send()
        .toBodyE { er =>
          SwarmError(s"Swarm download error $downloadURI: $er")
        }
        .leftSemiflatMap {
          case e @ SwarmError(errorMessage, _) ⇒
            Log[F].error(errorMessage) as e
        }
        .flatTap { r =>
          Log.eitherT[F, SwarmError].debug(s"Swarm download finished $downloadURI [${r.length}] bytes")
        }
  }

  def fetch(target: String)(implicit log: Log[F]): EitherT[F, SwarmError, fs2.Stream[F, ByteBuffer]] = {
    val downloadURI = uri(Bzz, target)
    Log.eitherT[F, SwarmError].debug(s"Swarm download started $downloadURI") *>
      sttp
        .response(asStream[fs2.Stream[F, ByteBuffer]])
        .get(downloadURI)
        .send()
        .toBodyE { er ⇒
          SwarmError(s"Error on downloading from $downloadURI. $er")
        }
        .leftSemiflatMap {
          case e @ SwarmError(errorMessage, _) ⇒
            Log[F].error(errorMessage) as e
        }
        .flatTap { r =>
          Log.eitherT[F, SwarmError].debug(s"Swarm download finished $downloadURI")
        }
  }

  /**
   * Upload a data.
   *
   * @see https://swarm-guide.readthedocs.io/en/latest/up-and-download.html
   * @return hash of resource (address in Swarm)
   */
  def upload(data: Array[Byte])(implicit log: Log[F]): EitherT[F, SwarmError, String] = {
    upload(ByteVector(data))
  }

  /**
   * Upload a data.
   *
   * @see https://swarm-guide.readthedocs.io/en/latest/up-and-download.html
   * @return hash of resource (address in Swarm)
   */
  def upload(data: ByteVector)(implicit log: Log[F]): EitherT[F, SwarmError, String] = {
    val uploadURI = uri(Bzz)
    Log.eitherT[F, SwarmError].info(s"Upload request. Data: $data") *>
      sttp
        .response(asString)
        .post(uploadURI)
        .body(data.toArray)
        .send()
        .toBodyE(er => SwarmError(s"Error on uploading to $uploadURI. $er"))
        .flatTap { r =>
          Log.eitherT[F, SwarmError].info(s"The resource has been uploaded.") *>
            Log.eitherT[F, SwarmError].debug(s"Resource size: ${r.length} bytes.")
        }
  }

  /**
   * Download a manifest directly.
   *
   * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#manifests
   * @param target hash of resource (file, metadata, manifest) or address from ENS
   *
   */
  def downloadRaw(target: String)(implicit log: Log[F]): EitherT[F, SwarmError, Manifest] = {
    val downloadURI = uri(BzzRaw, target)
    Log.eitherT[F, SwarmError].info(s"Download manifest request. Target: $target") *>
      sttp
        .response(asJson[Manifest])
        .get(downloadURI)
        .send()
        .toBodyE(er => SwarmError(s"Error on downloading manifest from $downloadURI. $er"))
        .flatMapF {
          case Left(er) ⇒
            Log[F].error(s"Deserialization error: $er") as
              SwarmError(s"Deserialization error on request to $downloadURI.", Some(er.error)).asLeft[Manifest]
          case Right(r) ⇒
            r.asRight[SwarmError].pure[F]
        }
        .flatTap { r =>
          Log.eitherT[F, SwarmError].info(s"A manifest has been downloaded.") *>
            Log.eitherT[F, SwarmError].debug(s"A raw manifest response: ${r.asJson}.")
        }
  }

  /**
   * Retrieve a mutable resource.
   *
   * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#retrieving-a-mutable-resource
   * @param target hash of resource (file, metadata, manifest) or address from ENS
   * @param param optional parameter (download concrete period or version or download the only metafile) for download
   * @return stored file or error if the file doesn't exist
   */
  def downloadMutableResource(
    target: String,
    param: Option[DownloadResourceParam] = None
  )(implicit log: Log[F]): EitherT[F, SwarmError, ByteVector] = {
    val downloadURI = uri(BzzResource, target, param.map(_.toParams).getOrElse(Nil))
    Log
      .eitherT[F, SwarmError]
      .info(s"Download a mutable resource request. Target: $target, param: ${param.getOrElse("<null>")}") *>
      sttp
        .response(asByteArray.map(ByteVector(_)))
        .get(downloadURI)
        .send()
        .toBodyE(er => SwarmError(s"Error on downloading raw from $downloadURI. $er"))
        .flatTap { r =>
          Log.eitherT[F, SwarmError].info(s"A mutable resource has been downladed. Size: ${r.size} bytes.")
        }
  }

  /**
   * Initialize a mutable resource. Upload a metafile with startTime, frequency and name, then upload data.
   * Period and version are set to 1 for initialization.
   *
   * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#creating-a-mutable-resource
   * @param mutableResourceId parameters that describe the mutable resource and required for searching updates of the mutable resource
   * @param data content the Mutable Resource will be initialized with
   * @param multiHash is a flag indicating whether the data field should be interpreted as a raw data or a multihash
   *                  TODO There is no implementation of multiHashed data for now.
   * @param signer signature algorithm. Must be ECDSA for real Swarm node
   * @return hash of metafile. This is the address of mutable resource
   */
  def initializeMutableResource(
    mutableResourceId: MutableResourceIdentifier,
    data: ByteVector,
    multiHash: Boolean,
    signer: Signer[ByteVector, ByteVector]
  )(implicit log: Log[F]): EitherT[F, SwarmError, String] =
    for {
      _ ← Log
        .eitherT[F, SwarmError]
        .info(
          s"Initialize a mutable resource. " +
            mutableResourceId.toString +
            s", data: ${data.size} bytes, " +
            s"multiHash: $multiHash"
        )
      req <- InitializeMutableResourceRequest(
        mutableResourceId,
        data,
        multiHash,
        signer
      )
      json = req.asJson
      _ <- Log.eitherT[F, SwarmError].debug(s"InitializeMutableResourceRequest: $json")
      resp <- sttp
        .response(asString.map(_.filter(_ != '"')))
        .post(uri(BzzResource))
        .body(jsonToBytes(json))
        .send()
        .toBodyE(er => SwarmError(s"Error on initializing a mutable resource. $er"))
      _ ← Log.eitherT[F, SwarmError].info(s"A mutable resource has been initialized. Hash: $resp")
    } yield resp

  /**
   * Upload a metafile for future use.
   *
   * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#creating-a-mutable-resource
   * @param mutableResourceId parameters that describe the mutable resource and required for searching updates of the mutable resource
   * @return hash of metafile. This is the address of mutable resource
   */
  def uploadMutableResource(
    mutableResourceId: MutableResourceIdentifier
  )(implicit log: Log[F]): EitherT[F, SwarmError, String] = {
    val req = UploadMutableResourceRequest(mutableResourceId)
    val json = req.asJson
    Log.eitherT[F, SwarmError].debug(s"UpdateMutableResourceRequest: $json") *>
      sttp
        .post(uri(BzzResource))
        .response(asString)
        .body(jsonToBytes(req.asJson))
        .send()
        .toBodyE(er => SwarmError(s"Error on uploading a mutable resource. $er"))
        .flatTap { r =>
          Log.eitherT[F, SwarmError].info(s"A metafile of a mutable resource has been uploaded. Hash: $r.")
        }
  }

  /**
   * Update a mutable resource.
   *
   * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#updating-a-mutable-resource
   * @param mutableResourceId parameters that describe the mutable resource and required for searching updates of the mutable resource
   * @param data content the Mutable Resource will be initialized with
   * @param multiHash is a flag indicating whether the data field should be interpreted as raw data or a multihash
   *                  TODO There is no implementation of multiHashed data for now.
   * @param period indicates for what period we are signing.
   *               It is a number, where `startTime + frequency * period >= currentTime`
   *               @see https://swarm-guide.readthedocs.io/en/latest/usage.html#mutable-resource-versioning
   * @param version indicates what resource version of the period we are signing
   * @param signer signature algorithm. Must be ECDSA for real Swarm node
   */
  def updateMutableResource(
    mutableResourceId: MutableResourceIdentifier,
    data: ByteVector,
    multiHash: Boolean,
    period: Int,
    version: Int,
    signer: Signer[ByteVector, ByteVector]
  )(implicit log: Log[F]): EitherT[F, SwarmError, Unit] =
    for {
      _ ← Log
        .eitherT[F, SwarmError]
        .info(
          s"Update a mutable resource. " +
            s"$mutableResourceId, " +
            s"data: ${data.size} bytes, " +
            s"multiHash: $multiHash, " +
            s"period: $period, " +
            s"version: $version"
        )
      req <- UpdateMutableResourceRequest(
        mutableResourceId,
        data,
        multiHash,
        period,
        version,
        signer
      )
      json = req.asJson
      _ ← Log.eitherT[F, SwarmError].debug(s"UpdateMutableResourceRequest: $json")
      updateURI = uri(BzzResource)
      response <- sttp
        .response(ignore)
        .post(updateURI)
        .body(jsonToBytes(json))
        .send()
        .toBodyE(er => SwarmError(s"Error on sending request to $updateURI. $er"))
      _ ← Log.eitherT[F, SwarmError].info("A mutable resource has been updated.")
    } yield response

}

object SwarmClient {

  def apply[F[_]: Monad: SttpStreamEffect](
    swarmUri: Uri,
    readTimeout: FiniteDuration
  ): SwarmClient[F] = {

    implicit val h: Hasher[ByteVector, ByteVector] = Keccak256Hasher.hasher

    new SwarmClient[F](swarmUri, readTimeout)
  }

  implicit class UnsafeClient(client: SwarmClient[IO]) {

    def uploadUnsafe(data: Array[Byte])(implicit log: Log[IO]): String =
      client.upload(data).value.unsafeRunSync().right.get

    def downloadUnsafe(target: String)(implicit log: Log[IO]): Array[Byte] =
      client.download(target).value.unsafeRunSync().right.get
  }
}
