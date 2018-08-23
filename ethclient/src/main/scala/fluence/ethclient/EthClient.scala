package fluence.ethclient

import java.util.concurrent.{CancellationException, CompletableFuture, CompletionException}

import cats.{ApplicativeError, Functor}
import cats.effect._
import cats.syntax.functor._
import cats.syntax.flatMap._
import org.web3j.protocol.core.{Ethereum, Request, Response}
import org.web3j.protocol.http.HttpService
import org.web3j.protocol.{Web3j, Web3jService}
import org.web3j.protocol.ipc.UnixIpcService

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
   * Returns the current client version.
   *
   * Method: """web3_clientVersion"""
   */
  def getClientVersion[F[_]: Async](): F[String] =
    request(_.web3ClientVersion()).map(_.getWeb3ClientVersion)

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
   * Uses http://localhost:8545/
   */
  def makeHttpResource[F[_]: Functor]()(implicit F: ApplicativeError[F, Throwable]): Resource[F, EthClient] =
    makeResource(new HttpService())

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
      )(_.shutdown())

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
