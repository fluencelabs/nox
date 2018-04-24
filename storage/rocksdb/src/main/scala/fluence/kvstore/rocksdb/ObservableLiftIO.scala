package fluence.kvstore.rocksdb

import cats.effect.{IO, LiftIO}
import monix.reactive.Observable

/**
 * [[LiftIO]] instance built for `monix.reactive.Observable`.
 */
trait ObservableLiftIO extends LiftIO[Observable] {
  override def liftIO[A](ioa: IO[A]): Observable[A] = Observable.fromIO(ioa)
}

object ObservableLiftIO {

  implicit val catsObservableLiftIO: ObservableLiftIO = new ObservableLiftIO {}

}
