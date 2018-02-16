import SbtCommons._

commons

grpc

libraryDependencies ++= Seq(
  shapeless,
  typeSafeConfig,
  ficus,
  slogging,
  scalatest
)