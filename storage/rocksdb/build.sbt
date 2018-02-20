import SbtCommons._

commons

libraryDependencies ++= Seq(
  rocksDb,
  typeSafeConfig,
  ficus,
  monix3,
  slogging,
  scalatest,
  mockito
)