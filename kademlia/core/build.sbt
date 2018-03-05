import SbtCommons._

enablePlugins(AutomateHeaderPlugin)

commons

libraryDependencies ++= Seq(
  cats1,
  slogging,
  scalatest,
  monix3 % Test
)