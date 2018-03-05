import SbtCommons._

enablePlugins(AutomateHeaderPlugin)

commons

grpc

libraryDependencies ++= Seq(
  monix3,
  scalatest
)