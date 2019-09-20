package fluence.vm

/**
 * Represents result of the VM invocation.
 *
 * @param output the computed result by Frank VM
 * @param spentGas spent gas by producing the output
 */
case class InvocationResult(
  output: Array[Byte],
  spentGas: Long
)
