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

  def compileFrankVMSettings(): Seq[Def.Setting[_]] =
    Seq(
      publishArtifact := false,
      test            := (test in Test).dependsOn(compile).value,
      compile := (compile in Compile)
        .dependsOn(Def.task {
          val log = streams.value.log
          log.info(s"Compiling Frank VM")

          compileFrank()
        })
        .value
    )

  def downloadLlamadb(): Seq[Def.Setting[_]] =
    Seq(
      publishArtifact := false,
      test            := (test in Test).dependsOn(compile).value,
      compile := (compile in Compile)
        .dependsOn(Def.task {
          // by defaults, user.dir in sbt points to a submodule directory while in Idea to the project root
          val resourcesPath =
            if (System.getProperty("user.dir").endsWith("/vm"))
              System.getProperty("user.dir") + "/src/it/resources/"
            else
              System.getProperty("user.dir") + "/vm/src/it/resources/"

          val log = streams.value.log
          val llamadbUrl = "https://github.com/fluencelabs/llamadb-wasm/releases/download/0.1.2/llama_db.wasm"
          val llamadbPreparedUrl =
            "https://github.com/fluencelabs/llamadb-wasm/releases/download/0.1.2/llama_db_prepared.wasm"

          log.info(s"Dowloading llamadb from $llamadbUrl to $resourcesPath")

          // -nc prevents downloading if file already exists
          val llamadbDownloadRet = s"wget -nc $llamadbUrl -O $resourcesPath/llama_db.wasm" !
          val llamadbPreparedDownloadRet = s"wget -nc $llamadbPreparedUrl -O $resourcesPath/llama_db_prepared.wasm" !

          // wget returns 0 of file was downloaded and 1 if file already exists
          assert(llamadbDownloadRet == 0 || llamadbDownloadRet == 1, s"Download failed: $llamadbUrl")
          assert(
            llamadbPreparedDownloadRet == 0 || llamadbPreparedDownloadRet == 1,
            s"Download failed: $llamadbPreparedUrl"
          )
        })
        .value
    )

  def prepareWorkerVM(): Seq[Def.Setting[_]] =
    Seq(
      publishArtifact := false,
      test            := (test in Test).dependsOn(compile).value,
      compile := (compile in Compile)
        .dependsOn(Def.task {
          println(s"OS is ${System.getProperty("os.name").toLowerCase}")
          System.getProperty("os.name").toLowerCase match {
            // in case of MacOS it needs to download library from bintray
            case mac if mac.contains("mac") => {
              // by defaults, user.dir in sbt points to a submodule directory while in Idea to the project root
              val resourcesPath =
                if (System.getProperty("user.dir").endsWith("/vm"))
                  // assuming that library has already built
                  System.getProperty("user.dir") + "/frank/target/release"
                else
                  System.getProperty("user.dir") + "/vm/frank/target/release"

              val log = streams.value.log
              val libfrankUrl = "https://dl.bintray.com/fluencelabs/releases/libfrank.so"

              log.info(s"Dowloading libfrank from $libfrankUrl to $resourcesPath")

              // -nc prevents downloading if file already exists
              val libfrankDownloadRet = s"wget -nc $libfrankUrl -O $resourcesPath/libfrank.so" !

              // wget returns 0 of file was downloaded and 1 if file already exists
              assert(libfrankDownloadRet == 0 || libfrankDownloadRet == 1, s"Download failed: $libfrankUrl")
            }
            // in case of *nix simply does nothing
            case linux if linux.contains("linux") => compileFrank()
            case osName =>
              throw new RuntimeException(s"$osName is unsupported, only *nix and MacOS OS are supported now")
          }

        })
        .value
    )
}
