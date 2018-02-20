import SbtCommons._

commons

libraryDependencies ++= Seq(
  scopt,
  fastParse,
  scalatest
)

mainClass := Some("fluence.client.ClientApp")