package fluence.crypto

import java.io.File
import java.nio.file.Files

import cats.MonadError
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.crypto.keypair.KeyPair
import io.circe.parser.decode
import io.circe.syntax._

import scala.language.higherKinds

class FileKeyStorage[F[_]](file: File)(implicit F: MonadError[F, Throwable]) {
  import KeyStore._
  def readKeyPair: F[KeyPair] = {
    val keyBytes = Files.readAllBytes(file.toPath)
    for {
      storageOp ← F.fromEither(decode[Option[KeyStore]](new String(keyBytes)))
      storage ← storageOp match {
        case None     ⇒ F.raiseError[KeyStore](new RuntimeException("Cannot parse file with keys."))
        case Some(ks) ⇒ F.pure(ks)
      }
    } yield storage.keyPair
  }

  def storeSecretKey(key: KeyPair): F[Unit] =
    F.catchNonFatal {
      if (!file.exists()) file.createNewFile() else throw new RuntimeException(file.getAbsolutePath + " already exists")
      val str = KeyStore(key).asJson.toString()

      Files.write(file.toPath, str.getBytes)
    }

  def getOrCreateKeyPair(f: ⇒ F[KeyPair]): F[KeyPair] =
    if (file.exists()) {
      readKeyPair
    } else {
      for {
        newKeys ← f
        _ ← storeSecretKey(newKeys)
      } yield newKeys
    }
}
