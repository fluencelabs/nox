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

package fluence.swarm

import cats.Monad
import cats.data.EitherT
import com.softwaremill.sttp._
import com.softwaremill.sttp.circe._
import fluence.crypto.Crypto.Hasher
import scodec.bits.ByteVector
import io.circe.syntax._
import cats.syntax.functor._
import fluence.swarm.crypto.Secp256k1Signer.Signer
import fluence.swarm.requests._
import fluence.swarm.responses.Manifest
import io.circe.{Json, Printer}

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

// TODO use pureConfig for parameters
// TODO implement extended swarm functions https://github.com/fluencelabs/dataengine/issues/52
// TODO split errors from Swarm and internal errors
// TODO add logs
/**
 * Client for working with Swarm.
 * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#
 *
 * @param host address of trusted swarm node
 * @param port port of trusted swarm node
 * @param hasher hashing algorithm. Must be Keccak SHA-3 algorithm for real Swarm node or another for test purposes
 *               @see https://en.wikipedia.org/wiki/SHA-3
 * @param sttpBackend way to represent the backend implementation.
 *                    Can be sync or async, with effects or not depending on the `F`
 */
class SwarmClient[F[_]: Monad](host: String, port: Int)(
  implicit sttpBackend: SttpBackend[F, Nothing],
  hasher: Hasher[ByteVector, ByteVector]
) extends slogging.LazyLogging {

  import fluence.swarm.helpers.ResponseOps._
  import BzzProtocol._

  // unpretty printer for http requests
  private val printer = Printer.noSpaces.copy(dropNullValues = true)

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
    uri"http://$host:$port".path(Seq(bzzProtocol.protocol, target) ++ path)

  private def uri(bzzUri: BzzProtocol) = uri"http://$host:$port".path(bzzUri.protocol)

  /**
   * Download a file.
   * @see https://swarm-guide.readthedocs.io/en/latest/up-and-download.html
   *
   * @param target hash of resource (file, metadata, manifest) or address from ENS.
   *
   */
  def download(target: String): EitherT[F, SwarmError, Array[Byte]] = {
    val downloadURI = uri(Bzz, target)
    logger.info(s"Download request. Target: $target")
    sttp
      .response(asByteArray)
      .get(downloadURI)
      .send()
      .toEitherT(er => SwarmError(s"Error on downloading from $downloadURI. $er"))
      .map { r =>
        logger.info(s"The resource has been downloaded.")
        logger.debug(s"Resource size: ${r.length} bytes.")
        r
      }
  }

  /**
   * Upload a data.
   * @see https://swarm-guide.readthedocs.io/en/latest/up-and-download.html
   *
   * @return hash of resource (address in Swarm)
   */
  def upload(data: ByteVector): EitherT[F, SwarmError, String] = {
    val uploadURI = uri(Bzz)
    logger.info(s"Upload request. Data: $data")
    sttp
      .response(asString)
      .post(uploadURI)
      .body(data.toArray)
      .send()
      .toEitherT(er => SwarmError(s"Error on uploading to $uploadURI. $er"))
      .map { r =>
        logger.info(s"The resource has been uploaded.")
        logger.debug(s"Resource size: ${r.length} bytes.")
        r
      }
  }

  /**
   * Download a manifest directly.
   * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#manifests
   *
   * @param target hash of resource (file, metadata, manifest) or address from ENS
   *
   */
  def downloadRaw(target: String): EitherT[F, SwarmError, Manifest] = {
    val downloadURI = uri(BzzRaw, target)
    logger.info(s"Download manifest request. Target: $target")
    sttp
      .response(asJson[Manifest])
      .get(downloadURI)
      .send()
      .toEitherT(er => SwarmError(s"Error on downloading manifest from $downloadURI. $er"))
      .subflatMap(_.left.map{er =>
        logger.error(s"Deserialization error: $er")
        SwarmError(s"Deserialization error on request to $downloadURI.", Some(er.error))
      })
      .map { r =>
        logger.info(s"A manifest has been downloaded.")
        logger.debug(s"A raw manifest response: ${r.asJson}.")
        r
      }
  }

  /**
   * Retrieve a mutable resource.
   * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#retrieving-a-mutable-resource
   *
   * @param target hash of resource (file, metadata, manifest) or address from ENS
   * @param param optional parameter (download concrete period or version or download the only metafile) for download
   * @return stored file or error if the file doesn't exist
   */
  def downloadMutableResource(
    target: String,
    param: Option[DownloadResourceParam]
  ): EitherT[F, SwarmError, ByteVector] = {
    val downloadURI = uri(BzzResource, target, param.map(_.toParams).getOrElse(Nil))
    logger.info(s"Download a mutable resource request. Target: $target, param: ${param.getOrElse("<null>")}")
    sttp
      .response(asByteArray.map(ByteVector(_)))
      .get(downloadURI)
      .send()
      .toEitherT(er => SwarmError(s"Error on downloading raw from $downloadURI. $er"))
      .map { r =>
        logger.info(s"A mutable resource has been downladed. Size: ${r.size} bytes.")
        r
      }
  }

  /**
   * Initialize a mutable resource. Upload a metafile with startTime, frequency and name, then upload data.
   * Period and version are set to 1 for initialization.
   * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#creating-a-mutable-resource
   *
   * @param name optional resource name. You can use any name.
   * @param frequency expected time interval between updates, in seconds
   * @param startTime time the resource is valid from, in Unix time (seconds). Set to the current epoch
   *                  You can also put a startTime in the past or in the future.
   *                  Setting it in the future will prevent nodes from finding content until the clock hits startTime.
   *                  Setting it in the past allows you to create a history for the resource retroactively
   * @param ownerAddr Swarm address (Ethereum wallet address)
   * @param data content the Mutable Resource will be initialized with
   * @param multiHash is a flag indicating whether the data field should be interpreted as a raw data or a multihash
   *                  TODO There is no implementation of multiHashed data for now.
   * @param signer signature algorithm. Must be ECDSA for real Swarm node
   * @return hash of metafile. This is the address of mutable resource
   */
  def initializeMutableResource(
    name: Option[String],
    frequency: FiniteDuration,
    startTime: FiniteDuration,
    ownerAddr: ByteVector,
    data: ByteVector,
    multiHash: Boolean,
    signer: Signer[ByteVector, ByteVector]
  ): EitherT[F, SwarmError, String] = {
    logger.info(
      s"Initialize a mutable resource. " +
        s"Name: ${name.getOrElse("<null>")}, " +
        s"fequency: $frequency, " +
        s"startTime: $startTime, " +
        s"owner: 0x${ownerAddr.toHex}, " +
        s"data: ${data.size} bytes, " +
        s"multiHash: $multiHash"
    )
    for {
      req <- InitializeMutableResourceRequest(
        name,
        frequency,
        startTime,
        ownerAddr,
        data,
        multiHash,
        signer
      )
      json = req.asJson
      _ = logger.debug(s"InitializeMutableResourceRequest: $json")
      resp <- sttp
        .response(asString.map(_.filter(_ != '"')))
        .post(uri(BzzResource))
        .body(jsonToBytes(json))
        .send()
        .toEitherT(er => SwarmError(s"Error on initializing a mutable resource. $er"))
      _ = logger.info(s"A mutable resource has been initialized. Hash: $resp")
    } yield resp
  }

  /**
   * Upload a metafile for future use.
   * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#creating-a-mutable-resource
   *
   * @param name optional resource name. You can use any name.
   * @param frequency expected time interval between updates, in seconds
   * @param startTime time the resource is valid from, in Unix time (seconds). Set to the current epoch
   *                  You can also put a startTime in the past or in the future.
   *                  Setting it in the future will prevent nodes from finding content until the clock hits startTime.
   *                  Setting it in the past allows you to create a history for the resource retroactively
   * @param ownerAddr Swarm address (Ethereum wallet address)
   * @return hash of metafile. This is the address of mutable resource
   */
  def uploadMutableResource(
    name: Option[String],
    frequency: FiniteDuration,
    startTime: FiniteDuration,
    ownerAddr: ByteVector
  ): EitherT[F, SwarmError, String] = {

    val req = UploadMutableResourceRequest(name, frequency, startTime, ownerAddr)
    val json = req.asJson
    logger.debug(s"UpdateMutableResourceRequest: $json")
    sttp
      .post(uri(BzzResource))
      .response(asString)
      .body(jsonToBytes(req.asJson))
      .send()
      .toEitherT(er => SwarmError(s"Error on uploading a mutable resource. $er"))
      .map { r =>
        logger.info(s"A metafile of a mutable resource has been uploaded. Hash: $r.")
        r
      }
  }

  /**
   * Update a mutable resource.
   * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#updating-a-mutable-resource
   *
   * @param name optional resource name. You can use any name
   * @param frequency expected time interval between updates, in seconds
   * @param startTime time the resource is valid from, in Unix time (seconds). Set to the current epoch
   *                  You can also put a startTime in the past or in the future.
   *                  Setting it in the future will prevent nodes from finding content until the clock hits startTime.
   *                  Setting it in the past allows you to create a history for the resource retroactively
   * @param ownerAddr Swarm address (Ethereum wallet address)
   * @param data content the Mutable Resource will be initialized with
   * @param multiHash is a flag indicating whether the data field should be interpreted as raw data or a multihash
   *                  TODO There is no implementation of multiHashed data for now.
   * @param period indicates for what period we are signing. Depending on the current time, startTime and frequency
   * @param version indicates what resource version of the period we are signing
   * @param signer signature algorithm. Must be ECDSA for real Swarm node
   */
  def updateMutableResource(
    name: Option[String],
    frequency: FiniteDuration,
    startTime: FiniteDuration,
    ownerAddr: ByteVector,
    data: ByteVector,
    multiHash: Boolean,
    period: Int,
    version: Int,
    signer: Signer[ByteVector, ByteVector]
  ): EitherT[F, SwarmError, Unit] = {
    logger.info(
      s"Update a mutable resource. " +
        s"Name: ${name.getOrElse("<null>")}, " +
        s"fequency: $frequency, " +
        s"startTime: $startTime, " +
        s"owner: 0x${ownerAddr.toHex}, " +
        s"data: ${data.size} bytes, " +
        s"multiHash: $multiHash, " +
        s"period: $period, " +
        s"version: $version"
    )
    for {
      req <- UpdateMutableResourceRequest(
        name,
        frequency,
        startTime,
        ownerAddr,
        data,
        multiHash,
        period,
        version,
        signer
      )
      json = req.asJson
      _ = logger.debug(s"UpdateMutableResourceRequest: $json")
      updateURI = uri(BzzResource)
      response <- EitherT(
        sttp
          .response(ignore)
          .post(updateURI)
          .body(jsonToBytes(json))
          .send()
          .map(_.body)
      ).leftMap(er => SwarmError(s"Error on sending request to $updateURI. $er"))
      _ = logger.info("A mutable resource has been updated.")
    } yield response

  }
}
