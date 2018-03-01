import SbtCommons._
import com.typesafe.sbt.packager.docker._

// see docker plugin docs https://www.scala-sbt.org/sbt-native-packager/formats/docker.html
enablePlugins(JavaAppPackaging, DockerPlugin)

commons

libraryDependencies ++= Seq(
  catsFree,
  scopt,
  fastParse,
  jline,
  scalatest
)

mainClass := Some("fluence.client.ClientApp")

packageName in Docker := "fluencelabs/client"

dockerCommands ++= Seq(
  Cmd("ENV", "FLUENCE_GIT_HASH", sys.process.Process("git rev-parse HEAD").lineStream_!.head),
  Cmd("ENV", "FLUENCE_KEYS_PATH", "/etc/fluence/keys")
)

dockerExposedVolumes := Seq("/etc/fluence")

version in Docker := sys.process.Process("git rev-parse HEAD").lineStream_!.head

dockerUpdateLatest := (sys.process.Process("git rev-parse --abbrev-ref HEAD").lineStream_!.head.trim == "master")