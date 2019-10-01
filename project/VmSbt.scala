import sbt.Keys.{compile, publishArtifact, streams, test}
import sbt.internal.util.ManagedLogger
import sbt.{Def, file, _}
import SbtCommons.{download, foldNixMac}

import scala.sys.process._

object VmSbt {
  val compileFrank = TaskKey[Unit]("compileFrank")
  val downloadLlama = TaskKey[Unit]("downloadLlama")
  val makeFrankSo = TaskKey[Unit]("makeFrank")

  private def doCompileFrank(vmDirectory: sbt.File)(implicit log: ManagedLogger): Unit = {
    val frankFolder = vmDirectory / "frank"
    val libName = foldNixMac("libfrank.so", "libfrank.dylib")
    val libPath = frankFolder / "target" / "release" / libName
    if (libPath.exists()) {
      log.info("libfrank.so already exists, won't compile")
    } else {
      val compileCmd =
        s"cargo +nightly-2019-09-23 build --manifest-path ${frankFolder.absolutePath}/Cargo.toml --release"

      log.info(s"Compiling Frank VM")
      assert((compileCmd !) == 0, "Frank VM compilation failed")
    }
  }

  private def downloadFrankSo(vmDirectory: sbt.File)(implicit log: ManagedLogger): Unit = {
    val soPath = vmDirectory / "frank" / "target" / "release" / "libfrank.so"
    val libfrankUrl = "https://dl.bintray.com/fluencelabs/releases/libfrank.so"

    download(libfrankUrl, soPath)
  }

  def compileFrank(vmDirectory: SettingKey[sbt.File]) = Def.task {
    doCompileFrank(vmDirectory.value)(streams.value.log)
  }

  def downloadLlama(resourcesDir: SettingKey[sbt.File]) = Def.task {
    implicit val log = streams.value.log
    val resourcesPath = resourcesDir.value
    val llamadbUrl = "https://github.com/fluencelabs/llamadb-wasm/releases/download/0.1.2/llama_db.wasm"
    val llamadbPreparedUrl =
      "https://github.com/fluencelabs/llamadb-wasm/releases/download/0.1.2/llama_db_prepared.wasm"

    download(llamadbUrl, resourcesPath / "llama_db.wasm")
    download(llamadbPreparedUrl, resourcesPath / "llama_db_prepared.wasm")
  }

  def makeFrankSoLib(vmDirectory: SettingKey[sbt.File]) = Def.task {
    implicit val log = streams.value.log

    // on *nix, compile frank to .so; on MacOS, download library from bintray
//    foldNixMac(nix = doCompileFrank(vmDirectory.value), mac = downloadFrankSo(vmDirectory.value))
    foldNixMac(nix = downloadFrankSo(vmDirectory.value), mac = downloadFrankSo(vmDirectory.value))
  }
}
