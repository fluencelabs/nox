import SbtCommons._
import com.typesafe.sbt.packager.docker._

enablePlugins(JavaAppPackaging, DockerPlugin)

commons

protobuf

libraryDependencies ++= Seq(
  scalatest
)

mainClass := Some("fluence.node.NodeApp")

packageName in Docker := "fluencelabs/node"

dockerCommands ++= Seq(
  Cmd("ENV", "FLUENCE_GIT_HASH", sys.process.Process("git rev-parse HEAD").lineStream_!.head)
)

version in Docker := sys.process.Process("git rev-parse HEAD").lineStream_!.head

dockerUpdateLatest := (sys.process.Process("git rev-parse --abbrev-ref HEAD").lineStream_!.head.trim == "master")