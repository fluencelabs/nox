import SbtCommons._

enablePlugins(AutomateHeaderPlugin)

commons

libraryDependencies ++= Seq(
  typeSafeConfig,
  ficus,
  monix3,
  slogging,
  scalatest
)