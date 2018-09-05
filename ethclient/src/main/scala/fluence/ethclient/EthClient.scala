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
import java.util.concurrent.{CancellationException, CompletableFuture, CompletionException}

import cats.{ApplicativeError, Functor}
import cats.effect._
import cats.syntax.functor._
import cats.syntax.flatMap._
import org.web3j.abi.EventEncoder
import org.web3j.protocol.core.methods.request.EthFilter
import org.web3j.protocol.core.methods.response.Log
import org.web3j.protocol.core.{DefaultBlockParameterName, Ethereum, Request, Response}
import org.web3j.protocol.http.HttpService
import org.web3j.protocol.{Web3j, Web3jService}
import org.web3j.protocol.ipc.UnixIpcService
import rx.Observable

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
class EthClient private (private val web3: Web3j) {

  import EthClient.FromJavaFuture

  /**
   * Returns the current client version.
   *
   * Method: """web3_clientVersion"""
   */
  def clientVersion[F[_]: Async](): F[String] =
    request(_.web3ClientVersion()).map(_.getWeb3ClientVersion)

  /**
   * Returns the current client version.
   *
   * Method: """web3_clientVersion"""
   */
  def web3sha3[F[_]: Async](data: String): F[String] =
    request(_.web3Sha3(data)).map { s ⇒
      if (s.hasError) println(s.getError.getCode + " - " + s.getError.getMessage)
      s.getResult
    }

  /**
   * Subscribe to logs topic, calling back each time the log message matches.
   * As Rx API doesn't really offer us backpressure support, streaming is encapsulated.
   *
   * @param contractAddress Contract address to watch events on
   * @param topic Logs topic, use [[EventEncoder]] to prepare it
   * @param callback Function to be called for each incoming log message
   * @tparam F Effect
   * @return Unsubscribe callback
   */
  def subscribeToLogsTopic[F[_]: Sync, G[_]: Effect](
    contractAddress: String,
    topic: String,
    callback: Log ⇒ G[Unit]
  ): F[F[Unit]] =
    Sync[F]
      .delay(
        // Create a subscription. New subscription will be created for each call of F, so it's referentially transparent
        web3
          .ethLogObservable(
            new EthFilter(
              DefaultBlockParameterName.LATEST,
              DefaultBlockParameterName.LATEST,
              Collections.emptyList[String]()
            ).addSingleTopic(topic)
          )
          .flatMap({ log ⇒
            println("yet we;re inside")
            println(log)

            // TODO: must provide all the error callbacks as well
            // Convert callback to a Java Future, then convert it to Observable
            val future = new CompletableFuture[Unit]()

            Effect[G].toIO(callback(log)).unsafeRunAsync {
              case Left(err) ⇒ future.completeExceptionally(err)
              case _ ⇒ future.complete(())
            }

            Observable.from(future)
          })
          .subscribe()
      )
      .map(
        subscription ⇒
          Sync[F].delay(
            subscription.unsubscribe()
        )
      )

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
  def makeHttpResource[F[_]: Functor](
    url: Option[String] = None
  )(implicit F: ApplicativeError[F, Throwable]): Resource[F, EthClient] =
    makeResource(new HttpService(url.getOrElse(HttpService.DEFAULT_URL)))

  /**
   * Make a cats-effect's [[Resource]] for an [[EthClient]], encapsulating its acquire and release lifecycle steps.
   *
   * @param ipcSocketPath Full path to geth.ipc
   */
  def makeUnixIpcResource[F[_]: Functor](
    ipcSocketPath: String
  )(implicit F: ApplicativeError[F, Throwable]): Resource[F, EthClient] =
    makeResource(new UnixIpcService(ipcSocketPath))

  /**
   * Make a cats-effect's [[Resource]] for an [[EthClient]], encapsulating its acquire and release lifecycle steps.
   * Initiation and shutting down is done eagerly on the caller thread.
   *
   * @param service [[Web3j]]'s connection description
   */
  private def makeResource[F[_]: Functor](
    service: Web3jService
  )(implicit F: ApplicativeError[F, Throwable]): Resource[F, EthClient] =
    Resource
      .make(
        F.catchNonFatal(
          new EthClient(Web3j.build(service))
        )
      )(_ => /* _.shutdown()*/ F.pure(Unit))

  /**
   * Utility converters from Java's CompletableFuture to cats-effect types
   */
  private[ethclient] implicit class FromJavaFuture[A](cf: CompletableFuture[A]) {

    /**
     * Convert Java Future to some Cancelable F type.
     * It requires F to have Concurrent typeclass, which is a more strict requirement then just Async.
     */
    def asCancelable[F[_]: Concurrent]: F[A] =
      Concurrent[F].cancelable { cb =>
        cf.handle[Unit] { (result: A, err: Throwable) =>
          err match {
            case null =>
              cb(Right(result))
            case _: CancellationException =>
              ()
            case ex: CompletionException if ex.getCause ne null =>
              cb(Left(ex.getCause))
            case ex =>
              cb(Left(ex))
          }
        }
        IO(cf.cancel(true))
      }

    /**
     * Convert Java Future to some Async F type.
     * It requires F to be just Async.
     */
    def asAsync[F[_]: Async]: F[A] =
      Async[F].async { cb ⇒
        cf.handle[Unit] { (result: A, err: Throwable) ⇒
          err match {
            case null =>
              cb(Right(result))
            case _: CancellationException =>
              ()
            case ex: CompletionException if ex.getCause ne null =>
              cb(Left(ex.getCause))
            case ex =>
              cb(Left(ex))
          }
        }
      }
  }
}
