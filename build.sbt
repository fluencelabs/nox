import SbtCommons._
import sbt.Keys._
import sbt._

name := "dataengine"

commons

/* Projects */

lazy val statemachine = (project in file("statemachine"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
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
