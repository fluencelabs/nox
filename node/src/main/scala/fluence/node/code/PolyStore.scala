package fluence.node.code
import java.nio.file.Path

import cats.Monad
import cats.data.EitherT
import cats.effect.{LiftIO, Timer}
import fluence.effects.castore.{ContentAddressableStore, StoreError}
import fluence.node.eth.state.StorageRef
import fluence.node.eth.state.StorageType.StorageType

import scala.language.higherKinds

class PolyStore[F[_]: Timer: LiftIO: Monad](map: StorageType => ContentAddressableStore[F]) {

  def fetchTo(ref: StorageRef, dest: Path): EitherT[F, StoreError, Unit] = {
    map(ref.storageType).fetchTo(ref.storageHash, dest)
  }
}
