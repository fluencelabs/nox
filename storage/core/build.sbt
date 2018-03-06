import SbtCommons._

enablePlugins(AutomateHeaderPlugin)

commons

libraryDependencies ++= Seq(
  scalatest,
  monix3 % Test
)