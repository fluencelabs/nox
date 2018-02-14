import SbtCommons._

commons

libraryDependencies ++= Seq(
  cats1,
  slogging,
  scalatest,
  monix3 % Test
)