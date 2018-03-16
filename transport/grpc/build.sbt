import SbtCommons._

enablePlugins(AutomateHeaderPlugin)

commons

grpc

libraryDependencies ++= Seq(
  catsEffect,
  shapeless,
  typeSafeConfig,
  ficus,
  slogging,
  scalatest
)