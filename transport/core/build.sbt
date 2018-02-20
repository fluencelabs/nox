import SbtCommons._

commons

libraryDependencies ++= Seq(
  cats1,
  shapeless,
  slogging,
  "org.bitlet" % "weupnp" % "0.1.+",
  catsEffect
)