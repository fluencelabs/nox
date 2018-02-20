import SbtCommons._

commons

libraryDependencies ++= Seq(
  scopt,
  fastParse,
  jline,
  scalatest
)

mainClass := Some("fluence.client.ClientApp")