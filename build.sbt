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

val RocksDbV = "5.8.0"
val TypeSafeConfV = "1.3.2"
val FicusV = "1.4.2"
val MockitoV = "2.11.0"
val ScodecV = "1.10.3"

val logback = "ch.qos.logback" % "logback-classic" % "1.2.+"

val cats1 = "org.typelevel" %% "cats-core" % "1.0.0-MF"
val monix3 = "io.monix" %% "monix" % "3.0.0-M1"
val shapeless = "com.chuusai" %% "shapeless" % "2.3.+"

val rocksDb = "org.rocksdb" % "rocksdbjni" % RocksDbV
val typeSafeConfig = "com.typesafe" % "config" % TypeSafeConfV
val ficus = "com.iheart" %% "ficus" % FicusV

val mockito = "org.mockito" % "mockito-core" % MockitoV % Test
val scalatest = "org.scalatest" %% "scalatest" % "3.0.+" % Test

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
    libraryDependencies ++= Seq(
      scalatest
    )
  ).aggregate(
    `hack`,
    `kademlia`,
    `network`,
    `storage`,
    `b-tree`,
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
    shapeless,
    typeSafeConfig,
    ficus,
    "org.bitlet" % "weupnp" % "0.1.+",
    scalatest
  )
).dependsOn(`kademlia`).aggregate(`kademlia`)

lazy val `storage` = project.in(file("storage"))
  .settings(
    scalaV,
    scalariformPrefs,
    libraryDependencies ++= Seq(
      rocksDb,
      typeSafeConfig,
      ficus,
      monix3,
      scalatest,
      mockito
    )
  )

lazy val `b-tree` = project.in(file("b-tree"))
  .settings(
    scalaV,
    scalariformPrefs,
    libraryDependencies ++= Seq(
      typeSafeConfig,
      ficus,
      monix3,
      scalatest
    )
  ).dependsOn(`storage`).aggregate(`storage`)