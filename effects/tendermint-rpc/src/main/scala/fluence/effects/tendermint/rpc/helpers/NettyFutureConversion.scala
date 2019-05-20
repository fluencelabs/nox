package fluence.effects.tendermint.rpc.helpers

import cats.effect.Async
import io.netty.util.concurrent.Future

import scala.language.higherKinds

object NettyFutureConversion {
  implicit class FromNettyFuture[A](nf: Future[A]) {
    def asAsync[F[_]: Async]: F[A] = Async[F].async{ cb =>
      nf.addListener((future: Future[_ >: A]) => {
        if (future.isSuccess) {
          cb(Right(future.getNow))
        } else {
          cb(Left(future.cause()))
        }
      })
    }
  }
}
