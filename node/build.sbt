import SbtCommons._
import com.typesafe.sbt.packager.docker._

// see docker plugin docs https://www.scala-sbt.org/sbt-native-packager/formats/docker.html
enablePlugins(JavaAppPackaging, DockerPlugin)

commons

protobuf

libraryDependencies ++= Seq(
  scalatest
)

mainClass := Some("fluence.node.NodeApp")

packageName in Docker := "fluencelabs/node"

dockerCommands ++= Seq(
  Cmd("ENV", "FLUENCE_GIT_HASH", sys.process.Process("git rev-parse HEAD").lineStream_!.head),
  Cmd("ENV", "FLUENCE_DIR", "/var/fluence"),
  Cmd("ENV", "FLUENCE_KEYS_PATH", "/etc/fluence/keys"),
  Cmd("ENV", "FLUENCE_PORT", "11022")
)

dockerExposedPorts := Seq(11022)

dockerExposedVolumes := Seq("/var/fluence", "/etc/fluence")

version in Docker := sys.process.Process("git rev-parse HEAD").lineStream_!.head

dockerUpdateLatest := (sys.process.Process("git rev-parse --abbrev-ref HEAD").lineStream_!.head.trim == "master")