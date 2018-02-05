package fluence.crypto.algorithm

import java.security.SecureRandom

import fluence.crypto.keypair.KeyPair

trait KeyGenerator[F[_]] {
  def generateKeyPair(random: SecureRandom): F[KeyPair]
  def generateKeyPair(): F[KeyPair]
}
