import SbtCommons._
import sbt.Keys._
import sbt._

import scala.sys.process._

name := "dataengine"

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
      "com.github.cretz.asmble" % "asmble-compiler" % "0.4.0-fl",
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
      log.info("  Compiling counter.rs to counter.wasm and running with Data Engine.")

      val scalaVer = scalaVersion.value.slice(0, scalaVersion.value.lastIndexOf("."))
      val projectRoot = file("").getAbsolutePath
      val cmd = s"sh vm/examples/runExample.sh counter $projectRoot $scalaVer"

      log.info(s"  Running $cmd")

      assert(cmd ! log == 0, "  Compile Rust to Wasm failed.")
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
      "biz.enef"               %% "slogging"      % "0.6.1",
      "org.scodec"             %% "scodec-bits"   % "1.1.5",
      "io.circe"               %% "circe-generic" % "0.9.3",
      "io.circe"               %% "circe-parser"  % "0.9.3",
      "com.github.jtendermint" % "jabci"          % "0.17.1",
      "org.bouncycastle"       % "bcpkix-jdk15on" % "1.56",
      "net.i2p.crypto"         % "eddsa"          % "0.3.0",
      scalaTest
    )
  )
  .enablePlugins(AutomateHeaderPlugin)
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
      pureConfig,
      scodecBits,
      scodecCore,
      web3jCrypto,
      cryptoHashing,
      scalaTest
    )
  )
  .enablePlugins(AutomateHeaderPlugin)
