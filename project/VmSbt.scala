import sbt.Keys.{compile, publishArtifact, streams, test}
import sbt.{Def, file, _}

import scala.sys.process._

object VmSbt {

  def compileFrank(): Unit = {
    val projectRoot = file("").getAbsolutePath
    val frankFolder = s"$projectRoot/vm/frank"
    val compileCmd = s"cargo +nightly-2019-09-23 build --manifest-path $frankFolder/Cargo.toml --release"

    assert((compileCmd !) == 0, "Frank VM compilation failed")
  }

  val compileFrankTask: Def.Initialize[Task[Unit]] = Def.task {
    streams.value.log.info(s"Compiling Frank VM")
    compileFrank()
  }

  def frankVMSettings(): Seq[Def.Setting[_]] =
    Seq(
      publishArtifact := false,
      test            := (test in Test).dependsOn(compile).value,
      compile         := (compile in Compile).dependsOn(compileFrankTask).value
    )

  def downloadLlamaTask(resourcesDir: SettingKey[sbt.File]) = Def.task {
    val log = streams.value.log
    val resourcesPath = resourcesDir.value
    val llamadbUrl = "https://github.com/fluencelabs/llamadb-wasm/releases/download/0.1.2/llama_db.wasm"
    val llamadbPreparedUrl =
      "https://github.com/fluencelabs/llamadb-wasm/releases/download/0.1.2/llama_db_prepared.wasm"

    log.info(s"Dowloading llamadb from $llamadbUrl to $resourcesPath")

    // -c prevents downloading if file already exists
    val llamadbDownloadRet = s"wget -qc $llamadbUrl -O $resourcesPath/llama_db.wasm" !
    val llamadbPreparedDownloadRet = s"wget -qc $llamadbPreparedUrl -O $resourcesPath/llama_db_prepared.wasm" !

    assert(llamadbDownloadRet == 0, s"Download failed: $llamadbUrl")
    assert(llamadbPreparedDownloadRet == 0, s"Download failed: $llamadbPreparedUrl")
  }

  def prepareWorkerVM(vmDirectory: sbt.File): Seq[Def.Setting[_]] =
    Seq(
      publishArtifact := false,
      test            := (test in Test).dependsOn(compile).value,
      compile := (compile in Compile)
        .dependsOn(Def.task {
          val log = streams.value.log
          val os = System.getProperty("os.name").toLowerCase
          log.info(s"OS is $os")

          os match {
            // on MacOS, download library from bintray
            case os if os.contains("mac") =>
              val soPath = vmDirectory / "frank" / "target" / "release" / "libfrank.so"
              val libfrankUrl = "https://dl.bintray.com/fluencelabs/releases/libfrank.so"
              log.info(s"Downloading libfrank from $libfrankUrl to $soPath")
              val libfrankDownloadRet = s"wget -qc $libfrankUrl -O $soPath" ! // -c skips downloading if file exists

              assert(libfrankDownloadRet == 0, s"Download failed: $libfrankUrl")

            // on *nix, compile frank to .so
            case os if os.contains("linux") => compileFrank()

            case os =>
              throw new RuntimeException(s"$os is unsupported, only *nix and MacOS OS are supported now")
          }
        })
        .value
    )
}
