import scalariform.formatter.preferences._

val scalariformPrefs = scalariformPreferences := scalariformPreferences.value
  .setPreference(AlignParameters, true)
  .setPreference(AlignSingleLineCaseStatements, true)
  .setPreference(DoubleIndentConstructorArguments, true)
  .setPreference(PreserveSpaceBeforeArguments, true)
  .setPreference(RewriteArrowSymbols, true)
  .setPreference(DoubleIndentConstructorArguments, true)
  .setPreference(DanglingCloseParenthesis, Preserve)

scalariformPrefs

name := "node"

version := "0.1"

val scalaV = scalaVersion := "2.12.3"

scalaV

scalacOptions += "-Ypartial-unification"

val FreesV = "0.3.1"
val MonixV = "2.3.0"
val Fs2V = "0.10.0-M6"

val logback = "ch.qos.logback" % "logback-classic" % "1.2.3"
val log4j2 = "org.apache.logging.log4j" %% "log4j-api-scala" % "2.9.1"
val scalatest = "org.scalatest" %% "scalatest" % "3.0.3" % Test
val frees = "io.frees" %% "freestyle" % FreesV
val monix = "io.monix" %% "monix" % MonixV
val monixCats = "io.monix" %% "monix-cats" % MonixV
val cats = "org.typelevel" %% "cats" % "0.9.0"

val cats1 = "org.typelevel" %% "cats-core" % "1.0.0-MF"
val monix3 = "io.monix" %% "monix" % "3.0.0-M1"
val fs2 = "co.fs2" %% "fs2-core" % Fs2V
val fs2io = "co.fs2" %% "fs2-io" % Fs2V

val paradise = addCompilerPlugin("org.scalameta" % "paradise" % "3.0.0-M10" cross CrossVersion.full)

val grpc = Seq(
  PB.targets in Compile := Seq(
    scalapb.gen() -> (sourceManaged in Compile).value
  ),
  libraryDependencies ++= Seq(
    "com.trueaccord.scalapb" %% "scalapb-runtime" % com.trueaccord.scalapb.compiler.Version.scalapbVersion % "protobuf",
    "io.grpc" % "grpc-netty" % com.trueaccord.scalapb.compiler.Version.grpcJavaVersion,
    "com.trueaccord.scalapb" %% "scalapb-runtime-grpc" % com.trueaccord.scalapb.compiler.Version.scalapbVersion
  )
)

lazy val `fluence` = project.in(file("."))
  .settings(
    scalaV,
    paradise,
    libraryDependencies ++= Seq(
      frees,
      monix,
      monixCats,
      scalatest
    )
  )

lazy val `hack` = project.in(file("hack"))
  .settings(
    scalaV,
    libraryDependencies ++= Seq(
      "net.sf.ntru" % "ntru" % "1.2",
      scalatest
    )
  )

lazy val `kademlia` = project.in(file("kademlia"))
  .settings(
    scalaV,
    scalariformPrefs,
    libraryDependencies ++= Seq(
      cats1,
      logback,
      scalatest,
      monix3 % Test
    )
  )

lazy val `network` = project.in(file("network"))
.settings(
  scalaV,
  scalariformPrefs,
  grpc,
  libraryDependencies ++= Seq(
    monix3,
    "org.bitlet" % "weupnp" % "0.1.+",
    scalatest
  )
).dependsOn(`kademlia`).aggregate(`kademlia`)