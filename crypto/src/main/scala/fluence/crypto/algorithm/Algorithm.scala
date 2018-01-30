package fluence.crypto.algorithm

import java.security.SecureRandom

import fluence.crypto.keypair.KeyPair

trait Algorithm {
  def generateKeyPair(random: SecureRandom): KeyPair
  def generateKeyPair(): KeyPair
}
