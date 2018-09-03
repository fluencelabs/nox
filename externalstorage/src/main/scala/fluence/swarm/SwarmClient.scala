/*
 * Copyright (C) 2018  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
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
import fluence.swarm.ECDSASigner.Signer
import fluence.swarm.requests._
import fluence.swarm.responses.RawResponse
import io.circe.{Json, Printer}

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
 * @param hasher hashing algorithm. Must be Keccak SHA-3 algorithm for real Swarm node.
 */
class SwarmClient[F[_]: Monad](host: String, port: Int)(
  implicit sttpBackend: SttpBackend[F, Nothing],
  hasher: Hasher[ByteVector, ByteVector]
) extends slogging.LazyLogging {

  import fluence.swarm.helpers.ResponseOps._

  // unpretty printer for http requests
  private val printer = Printer.noSpaces.copy(dropNullValues = true)

  // generate body from json for http requests
  private def genBody(json: Json) = printer.pretty(json).getBytes

  /**
   * Generate uri for requests.
   *
   * @param bzzProtocol protocol for requests: `bzz:/`, `bzz-resource:/`, etc
   * @param target Hash of resource (file, metadata, manifest) or address from ENS.
   * @param path additional parameters for request
   * @return generated uri
   */
  private def uri(bzzProtocol: String, target: String, path: Seq[String] = Nil) =
    uri"http://$host:$port".path(Seq(bzzProtocol, target) ++ path)
  private def uri(bzzUri: String) = uri"http://$host:$port".path(bzzUri)

  /**
   * Download a file.
   * @see https://swarm-guide.readthedocs.io/en/latest/usage.html
   *
   * @param target Hash of resource (file, metadata, manifest) or address from ENS.
   *
   */
  def download[T](target: String): EitherT[F, SwarmError, Array[Byte]] = {
    val downloadURI = uri("bzz:", target)
    logger.info(s"Download request. Target: $target")
    sttp
      .response(asByteArray)
      .get(downloadURI)
      .send()
      .toEitherT(er => SwarmError(s"Error on downloading from $downloadURI. $er"))
      .map { r =>
        logger.info(s"A resource downladed.")
        logger.debug(s"Resource size: ${r.size} bytes.")
        r
      }
  }

  /**
   * Download a manifest directly.
   * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#manifests
   *
   * @param target Hash of resource (file, metadata, manifest) or address from ENS.
   *
   */
  def downloadRaw[T](target: String): EitherT[F, SwarmError, RawResponse] = {
    val downloadURI = uri("bzz-raw:", target)
    logger.info(s"Download manifest request. Target: $target")
    sttp
      .response(asJson[RawResponse])
      .get(downloadURI)
      .send()
      .toEitherT(er => SwarmError(s"Error on downloading manifest from $downloadURI. $er"))
      .flatMap(
        EitherT
          .fromEither(_)
          .leftMap(er => SwarmError(s"Deserialization error on request to $downloadURI.", Some(er.error)))
      )
      .map { r =>
        logger.info(s"A manifest downloaded.")
        logger.debug(s"A raw manifest response: ${r.asJson}.")
        r
      }
  }

  /**
   * Retrieve a mutable resource.
   * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#retrieving-a-mutable-resource
   *
   * @param target Hash of resource (file, metadata, manifest) or address from ENS.
   * @param param Optional parameter (download concrete period or version or download the only metafile) for download.
   * @return Stored file or error if the file doesn't exist.
   *
   */
  def downloadMRU(
    target: String,
    param: Option[DownloadResourceParam]
  ): EitherT[F, SwarmError, Array[Byte]] = {
    val downloadURI = uri("bzz-resource:", target, param.map(_.toParams).getOrElse(Nil))
    logger.info(s"Download a mutable resource request. Target: $target, param: ${param.getOrElse("<null>")}")
    sttp
      .response(asByteArray)
      .get(downloadURI)
      .send()
      .toEitherT(er => SwarmError(s"Error on downloading raw from $downloadURI. $er"))
      .map { r =>
        logger.info(s"A mutable resource downladed. Size: ${r.size} bytes.")
        r
      }
  }

  /**
   * Initialize a mutable resource. Upload a metafile with startTime, frequency and name, than upload data.
   * Period and version sets to 1 for initialization.
   * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#creating-a-mutable-resource
   *
   * @param name optional resource name. You can use any name.
   * @param frequency expected time interval between updates, in seconds
   * @param startTime time the resource is valid from, in Unix time (seconds). Set to the current epoch
   *                  You can also put a startTime in the past or in the future.
   *                  Setting it in the future will prevent nodes from finding content until the clock hits startTime.
   *                  Setting it in the past allows you to create a history for the resource retroactively.
   * @param ownerAddr Swarm address (Ethereum wallet address)
   * @param data content the Mutable Resource will be initialized with.
   * @param multiHash is a flag indicating whether the data field should be interpreted as raw data or a multihash.
   *                  TODO There is no implementation of multiHashed data for now.
   * @param signer signature algorithm. Must be ECDSA for real Swarm node.
   * @return hash of metafile. This is the address of mutable resource.
   */
  def initializeMRU(
    name: Option[String],
    frequency: Long,
    startTime: Long,
    ownerAddr: ByteVector,
    data: ByteVector,
    multiHash: Boolean,
    signer: Signer[ByteVector, ByteVector]
  ): EitherT[F, SwarmError, String] = {
    logger.info(
      s"Initialize a mutable resource. Name: ${name.getOrElse("<null>")}, fequency: $frequency, startTime: $startTime, " +
        s"owner: 0x${ownerAddr.toHex}, data: ${data.size} bytes, multiHash: $multiHash"
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
        .response(asString.map(_.replaceAll("\"", "")))
        .post(uri("bzz-resource:"))
        .body(genBody(json))
        .send()
        .toEitherT(er => SwarmError(s"Error on initializing a mutable resource. $er"))
      _ = logger.info(s"A mutable resource initialized. Hash: $resp")
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
   *                  Setting it in the past allows you to create a history for the resource retroactively.
   * @param ownerAddr Swarm address (Ethereum wallet address)
   * @return hash of metafile. This is the address of mutable resource.
   */
  def uploadMRU(
    name: Option[String],
    frequency: Long,
    startTime: Long,
    ownerAddr: ByteVector
  ): EitherT[F, SwarmError, String] = {

    val req = UploadMutableResourceRequest(name, frequency, startTime, ownerAddr)
    val json = req.asJson
    logger.debug(s"UpdateMutableResourceRequest: $json")
    sttp
      .post(uri("bzz-resource:"))
      .response(asString)
      .body(genBody(req.asJson))
      .send()
      .toEitherT(er => SwarmError(s"Error on uploading a mutable resource. $er"))
      .map { r =>
        logger.info(s"A metafile of a mutable resource uploaded. Hash: $r.")
        r
      }
  }

  /**
   * Update a mutable resource.
   * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#updating-a-mutable-resource
   *
   * @param name optional resource name. You can use any name.
   * @param frequency expected time interval between updates, in seconds
   * @param startTime time the resource is valid from, in Unix time (seconds). Set to the current epoch
   *                  You can also put a startTime in the past or in the future.
   *                  Setting it in the future will prevent nodes from finding content until the clock hits startTime.
   *                  Setting it in the past allows you to create a history for the resource retroactively.
   * @param ownerAddr Swarm address (Ethereum wallet address)
   * @param data content the Mutable Resource will be initialized with
   * @param multiHash is a flag indicating whether the data field should be interpreted as raw data or a multihash.
   *                  TODO There is no implementation of multiHashed data for now.
   * @param period Indicates for what period we are signing. Depending on the current time, startTime and frequency.
   * @param version Indicates what resource version of the period we are signing.
   * @param signer signature algorithm. Must be ECDSA for real Swarm node.
   */
  def updateMRU(
    name: Option[String],
    frequency: Long,
    startTime: Long,
    ownerAddr: ByteVector,
    data: ByteVector,
    multiHash: Boolean,
    period: Int,
    version: Int,
    signer: Signer[ByteVector, ByteVector]
  ): EitherT[F, SwarmError, Unit] = {
    logger.info(
      s"Update a mutable resource. Name: ${name.getOrElse("<null>")}, fequency: $frequency, " +
        s"startTime: $startTime, owner: 0x${ownerAddr.toHex}, data: ${data.size} bytes, multiHash: $multiHash, " +
        s"period: $period, version: $version"
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
      updateURI = uri("bzz-resource:")
      response <- EitherT(
        sttp
          .response(ignore)
          .post(updateURI)
          .body(genBody(json))
          .send()
          .map(_.body)
      ).leftMap(er => SwarmError(s"Error on sending request to $updateURI. $er"))
      _ = logger.info("A mutable resource updated.")
    } yield response

  }

}
