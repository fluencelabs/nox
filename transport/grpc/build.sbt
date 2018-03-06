import SbtCommons._

enablePlugins(AutomateHeaderPlugin)

commons

grpc

libraryDependencies ++= Seq(
  shapeless,
  typeSafeConfig,
  ficus,
  slogging,
  scalatest
)