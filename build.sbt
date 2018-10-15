import SbtCommons._
import sbt.Keys._
import sbt._

import scala.sys.process._

name := "fluence"

commons

initialize := {
  val _ = initialize.value // run the previous initialization
  val required = "1.8" // counter.wast cannot be run under Java 9. Remove this check after fixes.
  val current = sys.props("java.specification.version")
  assert(current == required, s"Unsupported JDK: java.specification.version $current != $required")
}

/* Projects */

lazy val vm = (project in file("vm"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "com.github.cretz.asmble" % "asmble-compiler" % "0.4.0-fl-fix",
      cats,
      catsEffect,
      pureConfig,
      cryptoHashing,
      scalaTest,
      mockito
    )
  )
  .enablePlugins(AutomateHeaderPlugin)

lazy val `vm-counter` = (project in file("vm/examples/counter"))
  .settings(
    commons,
    // we have to build fat jar because is not possible to simply run [[CounterRunner]]
    // with sbt (like sbt vm-counter/run) because sbt uses custom ClassLoader.
    // 'Asmble' code required for loading some classes (like RuntimeHelpers)
    // only with system ClassLoader.
    assemblyJarName in assembly := "counter.jar",
    // override `run` task
    run := {
      val log = streams.value.log
      log.info("Compiling counter.rs to counter.wasm and running with Fluence.")

      val scalaVer = scalaVersion.value.slice(0, scalaVersion.value.lastIndexOf("."))
      val projectRoot = file("").getAbsolutePath
      val cmd = s"sh vm/examples/run_example.sh counter $projectRoot $scalaVer"

      log.info(s"Running $cmd")

      assert(cmd ! log == 0, "Compile Rust to Wasm failed.")
    }
  )
  .dependsOn(vm)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `vm-llamadb` = (project in file("vm/examples/llamadb"))
  .settings(
    commons,
    assemblyJarName in assembly := "llamadb.jar",
    // override `run` task
    run := {
      val log = streams.value.log
      log.info("Compiling llamadb.rs to llama_db.wasm and running with Fluence.")

      val scalaVer = scalaVersion.value.slice(0, scalaVersion.value.lastIndexOf("."))
      val projectRoot = file("").getAbsolutePath
      val cmd = s"sh vm/examples/run_example.sh llama_db $projectRoot $scalaVer"

      log.info(s"Running $cmd")

      assert(cmd ! log == 0, "Compile Rust to Wasm failed.")
    }
  )
  .dependsOn(vm)
  .enablePlugins(AutomateHeaderPlugin)

lazy val statemachine = (project in file("statemachine"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      cats,
      catsEffect,
      circeGeneric,
      circeParser,
      pureConfig,
      cryptoHashing,
      slogging,
      scodecBits,
      "com.github.jtendermint" % "jabci"          % "0.17.1",
      "org.bouncycastle"       % "bcpkix-jdk15on" % "1.56",
      "net.i2p.crypto"         % "eddsa"          % "0.3.0",
      scalaTest
    ),
    test in assembly := {}, // TODO: remove this line after SBT issue fix
    imageNames in docker := Seq(ImageName("fluencelabs/solver")),
    dockerfile in docker := {
      // Run `sbt docker` to create image

      // The assembly task generates a fat JAR file
      val artifact: File = assembly.value
      val artifactTargetPath = s"/app/${artifact.name}"

      // Tendermint constants
      val tmVersion = "0.23.0"
      val tmDataRoot = "/tendermint"
      val tmBinaryArchive = s"tendermint_${tmVersion}_linux_amd64.zip"
      val tmBinaryUrl = s"https://github.com/tendermint/tendermint/releases/download/v$tmVersion/$tmBinaryArchive"
      val tmP2pPort = 26656
      val tmRpcPort = 26657
      val tmPrometheusPort = 26660

      // State machine constants
      val smDataRoot = "/statemachine"
      val smRunScript = s"$smDataRoot/run-node.sh"

      val vmDataRoot = "/vmcode"

      new Dockerfile {
        from("xqdocker/ubuntu-openjdk:jre-8")
        run("apt", "-yqq", "update")
        run("apt", "-yqq", "install", "wget", "curl", "jq", "unzip", "screen")
        run("wget", tmBinaryUrl)
        run("unzip", "-d", "/bin", tmBinaryArchive)

        expose(tmP2pPort)
        expose(tmRpcPort)
        expose(tmPrometheusPort)

        volume(tmDataRoot)
        volume(smDataRoot)
        volume(vmDataRoot)

        add(artifact, artifactTargetPath)

        entryPoint("bash", smRunScript, tmDataRoot, smDataRoot, artifactTargetPath)
      }
    }
  )
  .enablePlugins(AutomateHeaderPlugin)
  .enablePlugins(DockerPlugin)
  .dependsOn(vm)

lazy val externalstorage = (project in file("externalstorage"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      cats,
      catsEffect,
      sttp,
      sttpCirce,
      sttpCatsBackend,
      slogging,
      circeCore,
      circeGeneric,
      circeGenericExtras,
      pureConfig,
      scodecBits,
      scodecCore,
      web3jCrypto,
      cryptoHashing,
      scalaTest
    )
  )
  .enablePlugins(AutomateHeaderPlugin)

lazy val ethclient = (project in file("ethclient"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      web3jCore,
      slogging,
      scodecBits,
      cats,
      catsEffect,
      fs2,
      scalaTest
    ),
  )
  .enablePlugins(AutomateHeaderPlugin)

lazy val node = project
  .settings(
    commons,
    kindProjector,
    libraryDependencies ++= Seq(
      catsEffect,
      sttp,
      sttpCatsBackend,
      fs2io
    )
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(ethclient)
