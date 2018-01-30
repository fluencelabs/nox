import SbtCommons._

commons

libraryDependencies ++= Seq(
  typeSafeConfig,
  ficus,
  monix3,
  logback,
  scalatest
)