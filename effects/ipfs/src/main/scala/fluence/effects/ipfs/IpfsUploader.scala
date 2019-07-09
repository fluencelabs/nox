package fluence.effects.ipfs

import cats.data.EitherT
import fluence.effects.castore.StoreError
import fluence.log.Log
import scodec.bits.ByteVector

import scala.language.higherKinds

trait IpfsUploader[F[_]] {
  def upload[A: IpfsData](data: A)(implicit log: Log[F]): EitherT[F, StoreError, ByteVector]
}
