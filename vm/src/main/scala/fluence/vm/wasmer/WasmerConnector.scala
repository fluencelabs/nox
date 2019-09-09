package fluence.vm.wasmer


class WasmerConnector {
  /**
   * Invokes main module.
   *
   * @param arg argument for invoked module
   */
  @native def invoke(arg: Array[Byte]): Array[Byte]

  /**
   * Initialize execution environment with given file path.
   *
   * @param filePath path to a wasm file
   */
  @native def init(filePath: String): Int
}
