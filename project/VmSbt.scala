import sbt.Keys.{compile, publishArtifact, streams, test}
import sbt.internal.util.ManagedLogger
import sbt.{Def, file, _}
import SbtCommons.{download, foldNixMac}

import scala.sys.process._

object VmSbt {
  val compileFrank = TaskKey[Unit]("compiles frank")
  val downloadLlama = TaskKey[Unit]("downloads llamadb .wasm files")
  val compileFrankTask: Def.Initialize[Task[Unit]] = Def.task { compileFrank()(streams.value.log) }

  private def compileFrank()(implicit log: ManagedLogger): Unit = {
    val projectRoot = file("").getAbsolutePath
    val frankFolder = s"$projectRoot/vm/frank"
    val compileCmd = s"cargo +nightly-2019-09-23 build --manifest-path $frankFolder/Cargo.toml --release"

    log.info(s"Compiling Frank VM")
    assert((compileCmd !) == 0, "Frank VM compilation failed")
  }

  private def downloadFrankSo(vmDirectory: sbt.File)(implicit log: ManagedLogger): Unit = {
    val soPath = vmDirectory / "frank" / "target" / "release" / "libfrank.so"
    val libfrankUrl = "https://dl.bintray.com/fluencelabs/releases/libfrank.so"

    download(libfrankUrl, soPath)
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
//    foldNixMac(nix = compileFrank(), mac = downloadFrankSo(vmDirectory.value))
    foldNixMac(nix = downloadFrankSo(vmDirectory.value), mac = downloadFrankSo(vmDirectory.value))
  }
}
