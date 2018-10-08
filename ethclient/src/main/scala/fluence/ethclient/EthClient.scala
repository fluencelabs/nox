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

package fluence.ethclient

import java.util.Collections

import cats.ApplicativeError
import cats.effect._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.ethclient.helpers.JavaFutureConversion._
import org.web3j.abi.EventEncoder
import org.web3j.protocol.core._
import org.web3j.protocol.core.methods.request.SingleAddressEthFilter
import org.web3j.protocol.core.methods.response.Log
import org.web3j.protocol.http.HttpService
import org.web3j.protocol.{Web3j, Web3jService}
import org.web3j.tx.ClientTransactionManager
import org.web3j.tx.gas.DefaultGasProvider
import slogging._
import fluence.ethclient.helpers.JavaRxToFs2._

import scala.language.higherKinds

/**
 * EthClient provides a functional end-user API for Ethereum (Light) Client, currently by calling Web3j methods.
 *
 * Web3j interacts with an Ethereum node, running either in full or light mode, via ipc, http,
 * or by any other supported transport.
 * Constructor is private in order to enforce class user to use Resource notation, provided in the companion object,
 * hence ensuring that allocated resources are closed properly.
 *
 * @param web3 Underlying Web3j instance
 */
class EthClient private (private val web3: Web3j) extends LazyLogging {

  /**
   * Returns the current client version.
   *
   * Method: """web3_clientVersion"""
   */
  def clientVersion[F[_]: Async](): F[String] =
    request(_.web3ClientVersion()).map(_.getWeb3ClientVersion)

  /**
   * Subscribe to logs topic, calling back each time the log message matches.
   * As Rx API doesn't really offer us backpressure support, streaming is encapsulated.
   *
   * @param contractAddress Contract address to watch events on
   * @param topic Logs topic, use [[EventEncoder]] to prepare it
   * @tparam F Effect
   * @return Unsubscribe callback
   */
  def subscribeToLogsTopic[F[_]: ConcurrentEffect](
    contractAddress: String,
    topic: String
  ): fs2.Stream[F, Log] =
    web3
      .ethLogObservable(
        new SingleAddressEthFilter(
          DefaultBlockParameterName.LATEST,
          DefaultBlockParameterName.LATEST,
          contractAddress
        ).addSingleTopic(topic)
      )
      .toFS2[F]

  /**
   * Instantiate ABI for existing Deployer contract at the address
   * @param at Address of the deployed contract
   * @param userAddress Address of the user sending transaction to the contract
   * @tparam F Effect
   * @return Instance of the Deployer contract
   */
  def getDeployer[F[_]: Async](
    at: String,
    userAddress: String
  ): F[Deployer] =
    Async[F].delay {
      val txManager = new ClientTransactionManager(web3, userAddress)
      val contract = Deployer.load(
        at,
        web3,
        txManager,
        DefaultGasProvider.GAS_PRICE,
        DefaultGasProvider.GAS_LIMIT
      )

      // TODO: check contract.isValid and throw an exception
      // currently it doesn't work because contract's binary code is different after deploy for some unknown reason
      // maybe it's web3j generator's fault
      contract
    }

  /**
   * Make an async request to Ethereum, lifting its response to an async F type.
   *
   * @param req Request on Ethereum API to be called
   * @tparam F Async effect type
   * @tparam T Response type
   */
  private def request[F[_]: Async, T <: Response[_]](req: Ethereum ⇒ Request[_, T]): F[T] =
    Sync[F]
      .delay(req(web3).sendAsync()) // sendAsync eagerly launches the call on some thread pool, so we delay it to make it referentially transparent
      .flatMap(_.asAsync[F])

  /**
   * Eagerly shuts down web3, so that no further calls are possible.
   * Intended to be used only by Resource's deallocation.
   */
  private def shutdown[F[_]]()(implicit F: ApplicativeError[F, Throwable]): F[Unit] =
    F.catchNonFatal(web3.shutdown())

}

object EthClient {

  /**
   * Make a cats-effect's [[Resource]] for an [[EthClient]], encapsulating its acquire and release lifecycle steps.
   * @param url optional url, http://localhost:8545/ is used by default
   */
  def makeHttpResource[F[_]](
    url: Option[String] = None
  )(implicit F: Sync[F]): Resource[F, EthClient] =
    makeResource(new HttpService(url.getOrElse(HttpService.DEFAULT_URL)))

  /**
   * Make a cats-effect's [[Resource]] for an [[EthClient]], encapsulating its acquire and release lifecycle steps.
   * Initiation and shutting down is done eagerly on the caller thread.
   *
   * @param service [[Web3j]]'s connection description
   */
  private def makeResource[F[_]](
    service: Web3jService
  )(implicit F: Sync[F]): Resource[F, EthClient] =
    Resource
      .make(
        F.catchNonFatal(
          new EthClient(Web3j.build(service))
        )
      )(web3j => F.delay(web3j.shutdown()))
}
