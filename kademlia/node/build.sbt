import SbtCommons._

commons

libraryDependencies ++= Seq(
  cats1,
  logback,
  scalatest,
  monix3 % Test
)