import SbtCommons._

commons

libraryDependencies ++= Seq(
  catsFree,
  scopt,
  fastParse,
  jline,
  scalatest
)

mainClass := Some("fluence.client.ClientApp")