import SbtCommons._

enablePlugins(AutomateHeaderPlugin)

commons

libraryDependencies ++= Seq(
  scalatestKit,
  monix3
)