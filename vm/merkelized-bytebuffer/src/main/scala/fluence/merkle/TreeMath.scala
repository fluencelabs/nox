package fluence.merkle

object TreeMath {

  // log2(input) for integers
  def log2(input: Int): Int = {
    val res = 31 - Integer.numberOfLeadingZeros(input)
    res
  }

  // 2 ^ power
  def power2(power: Int): Int = {
    1 << power
  }
}
