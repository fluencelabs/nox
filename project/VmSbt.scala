import sbt.Keys.streams
import sbt.internal.util.ManagedLogger
import sbt.{Def, _}

import scala.sys.process._

object VmSbt {
  val downloadLlama = TaskKey[Unit]("downloadLlama")

  /**
   * Downloads a file from uri to specified target
   * @param target Target path. Should be a regular file, can't be directory because `wget -O`
   *               only works with regular files. You will need `wget -P` for directory target.
   */
  private def download(uri: String, target: sbt.File)(implicit log: ManagedLogger): Unit = {
    if (!target.getParentFile.exists()) target.getParentFile.mkdirs()
    if (!target.exists()) {
      val path = target.absolutePath
      log.info(s"Downloading $uri to $path")
      assert(
        s"wget -q $uri -O $path".! == 0,
        s"Download from $uri to $path failed. Note that target should be a path to a file, not a directory."
      )
    } else {
      log.info(s"${target.getName} already exists, won't download.")
    }
  }

  def downloadLlama(resourcesDir: SettingKey[sbt.File]) = Def.task {
    implicit val log = streams.value.log
    val resourcesPath = resourcesDir.value
    val llamadbUrl = "https://github.com/fluencelabs/llamadb-wasm/releases/download/0.1.2/llama_db.wasm"

    download(llamadbUrl, resourcesPath / "llama_db.wasm")
  }
}
