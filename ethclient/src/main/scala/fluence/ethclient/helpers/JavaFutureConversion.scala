package fluence.ethclient.helpers
import java.util.concurrent.{CancellationException, CompletableFuture, CompletionException}

import cats.effect.Async

object JavaFutureConversion {

  /**
   * Utility converters from Java's CompletableFuture to cats-effect types
   */
  private[ethclient] implicit class FromJavaFuture[A](cf: CompletableFuture[A]) {

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
