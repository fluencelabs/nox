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

import java.util.concurrent.TimeUnit

import cats.data.EitherT
import cats.effect._
import cats.syntax.apply._
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.syntax.applicativeError._
import cats.{~>, Applicative, Functor, Monad}
import fluence.ethclient.data.{Block, Log}
import fluence.ethclient.helpers.JavaFutureConversion._
import fluence.ethclient.syntax._
import org.web3j.abi.EventEncoder
import org.web3j.protocol.core._
import org.web3j.protocol.core.methods.request.SingleAddressEthFilter
import org.web3j.protocol.core.methods.response.EthSyncing
import org.web3j.protocol.http.HttpService
import org.web3j.protocol.{Web3j, Web3jService}
import org.web3j.tx.{ClientTransactionManager, TransactionManager}
import org.web3j.tx.gas.{ContractGasProvider, DefaultGasProvider}
import slogging._
import okhttp3.OkHttpClient
import org.web3j.abi.datatypes.Event

import scala.concurrent.duration._
import scala.language.{existentials, higherKinds}

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
  type ContractLoader[T] = (String, Web3j, TransactionManager, ContractGasProvider) => T

  import EthClient.ioToF

  /**
   * Returns the current client version.
   *
   * Method: """web3_clientVersion"""
   */
  def clientVersion[F[_]: LiftIO: Functor]: EitherT[F, EthRequestError, String] =
    request(_.web3ClientVersion()).map(_.getWeb3ClientVersion)

  /**
   * Returns the last block number
   * @tparam F Effect
   */
  def getBlockNumber[F[_]: LiftIO: Functor]: EitherT[F, EthRequestError, BigInt] =
    request(_.ethBlockNumber()).map(_.getBlockNumber).map(BigInt(_))

  /**
   * Checking if ethereum node is syncing or not.
   */
  def isSyncing[F[_]: LiftIO: Functor]: EitherT[F, EthRequestError, EthSyncing.Result] =
    request(_.ethSyncing()).map(_.getResult)

  /**
   * Checks node for syncing status every `checkPeriod` until node is synchronized.
   */
  private def waitEthSyncing[F[_]: LiftIO: Monad: Timer](checkPeriod: FiniteDuration): F[Unit] =
    isSyncing[F].value.flatMap {
      case Right(resp: EthSyncing.Syncing) if resp.isSyncing =>
        logger.info(
          s"Ethereum node is syncing. Current block: ${resp.getCurrentBlock}, highest block: ${resp.getHighestBlock}"
        )
        logger.info(s"Waiting ${checkPeriod.toSeconds} seconds for next attempt.")
        Timer[F].sleep(checkPeriod) *> waitEthSyncing(checkPeriod)
      case _ ⇒
        logger.info(s"Ethereum node is synced up, stop waiting")
        Applicative[F].unit
    }

  /**
   * Subscribe to logs topic, calling back each time the log message matches.
   * As Rx API doesn't really offer us backpressure support, streaming is encapsulated.
   *
   * @param contractAddress Contract address to watch events on
   * @param topic Logs topic, use [[EventEncoder]] to prepare it
   * @tparam F Effect
   * @return Unsubscribe callback
   */
  def subscribeToLogsTopic[F[_]: ConcurrentEffect: Timer](
    contractAddress: String,
    topic: Event,
    onErrorRetryAfter: FiniteDuration = 1.second
  ): fs2.Stream[F, Log] =
    web3
      .ethLogFlowable(
        new SingleAddressEthFilter(
          DefaultBlockParameterName.LATEST,
          DefaultBlockParameterName.LATEST,
          contractAddress
        ).addSingleTopic(EventEncoder.encode(topic))
      )
      .toStreamRetrying(onErrorRetryAfter)
      .map(Log.apply)

  /**
   * Provides a stream of newly received blocks, with no transaction objects included
   *
   * @param onErrorRetryAfter In case the web3 stream is terminated, restart it after this timeout
   * @param fullTransactionObjects if true, provides transactions embedded in blocks, otherwise
   *                                    transaction hashes
   * @tparam F Effect
   * @return Stream of Raw Responses (if provided) and delayed Block data class (to avoid parsing it if it's not necessary)
   */
  def blockStream[F[_]: ConcurrentEffect: Timer](
    onErrorRetryAfter: FiniteDuration = EthRetryPolicy.Default.delayPeriod,
    fullTransactionObjects: Boolean = false
  ): fs2.Stream[F, (Option[String], F[Option[Block]])] =
    web3
      .blockFlowable(fullTransactionObjects)
      .toStreamRetrying(onErrorRetryAfter)
      .map(
        ethBlock ⇒
          Option(ethBlock.getRawResponse) →
            Sync[F].delay[Option[Block]](Some(Block(ethBlock.getBlock))).handleError { e: Throwable =>
              logger.error(s"Cannot encode block from ethereum: ${ethBlock.getBlock}")
              e.printStackTrace()
              None
          }
      )

  /**
   * Helper for retrieving a web3j-prepared contract
   *
   * @param contractAddress Address of the deployed contract
   * @param userAddress Address of the user sending transaction to the contract
   * @param load Contract.load
   * @tparam T Contract type
   * @return Contract ABI instance
   */
  def getContract[T](
    contractAddress: String,
    userAddress: String,
    load: ContractLoader[T]
  ): T = {
    val txManager = new ClientTransactionManager(web3, userAddress)

    // TODO: check contract.isValid and throw an exception
    // currently it doesn't work because contract's binary code is different after deploy for some unknown reason
    // maybe it's web3j generator's fault
    load(contractAddress, web3, txManager, new DefaultGasProvider)
  }

  /**
   * Make an async request to Ethereum, lifting its response to an async F type.
   *
   * @param req Request on Ethereum API to be called
   * @tparam F Effect type
   * @tparam T Response type
   */
  private def request[F[_]: LiftIO, T <: Response[_]](req: Ethereum ⇒ Request[_, T]): EitherT[F, EthRequestError, T] =
    IO(req(web3).sendAsync()) // sendAsync eagerly launches the call on web3's internal thread pool, so we delay it to make it referentially transparent
      .flatMap(_.asAsync[IO])
      .attemptT
      .leftMap(EthRequestError)
      .mapK(ioToF[F])

  /**
   * Eagerly shuts down web3, so that no further calls are possible.
   * Intended to be used only by Resource's deallocation.
   */
  private def shutdown[F[_]: LiftIO](): EitherT[F, EthShutdownError, Unit] =
    IO(web3.shutdown()).attemptT.leftMap(EthShutdownError).mapK(ioToF[F])

}

object EthClient extends LazyLogging {

  private[ethclient] implicit def ioToF[F[_]: LiftIO]: IO ~> F = new (IO ~> F) {
    override def apply[A](fa: IO[A]): F[A] = fa.to[F]
  }

  /**
   * Set timeouts to 90 seconds because a light node can respond slowly.
   *
   */
  private def createOkHttpClient = {
    val builder = new OkHttpClient.Builder()
    builder.connectTimeout(90, TimeUnit.SECONDS)
    builder.readTimeout(90, TimeUnit.SECONDS)
    builder.writeTimeout(90, TimeUnit.SECONDS)
    builder.build
  }

  /**
   * Make a cats-effect's [[Resource]] for an [[EthClient]], encapsulating its acquire and release lifecycle steps.
   * Waits for syncinc before giving the client to use.
   *
   * @param url optional url, http://localhost:8545/ is used by default
   * @param includeRaw Whether to include unparsed JSON strings in the web3j's response objects
   * @param checkSyncPeriod Period of querying the Ethereum node to check if it's in sync
   */
  def make[F[_]: Timer: Async](
    url: Option[String] = None,
    includeRaw: Boolean = false,
    checkSyncPeriod: FiniteDuration = 3.seconds
  ): Resource[F, EthClient] =
    makeResource(new HttpService(url.getOrElse(HttpService.DEFAULT_URL), createOkHttpClient, includeRaw)).evalMap {
      client ⇒
        client.waitEthSyncing(checkSyncPeriod).map(_ ⇒ client)
    }

  /**
   * Make a cats-effect's [[Resource]] for an [[EthClient]], encapsulating its acquire and release lifecycle steps.
   * Initiation and shutting down is done eagerly on the caller thread.
   *
   * @param service [[Web3j]]'s connection description
   */
  private def makeResource[F[_]: LiftIO: Functor](
    service: Web3jService
  ): Resource[F, EthClient] =
    Resource
      .make(
        IO(new EthClient(Web3j.build(service))).to[F]
      )(web3j => web3j.shutdown[F]().value.void)
}
