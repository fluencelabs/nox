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
      "com.github.jtendermint" % "jabci" % "0.17.1",
      "org.bouncycastle" % "bcpkix-jdk15on" % "1.56",
      "com.google.code.gson" % "gson" % "2.8.5"
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
