import SbtCommons._

commons

libraryDependencies ++= Seq(
  cats1,
  monix3 % Test,
  scalatest,
  mockito
)