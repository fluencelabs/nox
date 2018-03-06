import SbtCommons._

enablePlugins(AutomateHeaderPlugin)

commons

libraryDependencies ++= Seq(
  chill,
  shapeless,
  scalatest
)