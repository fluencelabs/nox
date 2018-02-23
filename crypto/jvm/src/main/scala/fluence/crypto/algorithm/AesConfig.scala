package fluence.crypto.algorithm

case class AesConfig(
    iterationCount: Int = 50,
    salt: String = "fluence"
)
