import SbtCommons._

commons

grpc

libraryDependencies ++= Seq(
  monix3,
  shapeless,
  typeSafeConfig,
  ficus,
  slogging,
  "org.bitlet" % "weupnp" % "0.1.+",
  scalatest
)