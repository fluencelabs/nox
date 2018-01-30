import SbtCommons._

commons

grpc

libraryDependencies ++= Seq(
  monix3,
  shapeless,
  typeSafeConfig,
  ficus,
  logback,
  "org.bitlet" % "weupnp" % "0.1.+",
  scalatest
)