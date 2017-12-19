import scalariform.formatter.preferences._

val scalariformPrefs = scalariformPreferences := scalariformPreferences.value
  .setPreference(AlignSingleLineCaseStatements, true)
  .setPreference(DoubleIndentConstructorArguments, true)
  .setPreference(PreserveSpaceBeforeArguments, true)
  .setPreference(RewriteArrowSymbols, true)
  .setPreference(DoubleIndentConstructorArguments, true)
  .setPreference(DanglingCloseParenthesis, Preserve)
  .setPreference(SpaceBeforeContextColon, true)
  .setPreference(NewlineAtEndOfFile, true)

name := "fluence"

version := "0.1"

val scalaV = scalaVersion := "2.12.4"

scalacOptions in Compile ++= Seq("-Ypartial-unification", "-Xdisable-assertions")

javaOptions in Test ++= Seq("-ea")

val commons = Seq(
  scalaV,
  scalariformPrefs,
  organizationName := "Fluence Labs Limited",
  organizationHomepage := Some(new URL("https://fluence.ai")),
  startYear := Some(2017),
  licenses += ("AGPL-3.0", new URL("http://www.gnu.org/licenses/agpl-3.0.en.html"))
)

commons

val RocksDbV = "5.8.0"
val TypeSafeConfV = "1.3.2"
val FicusV = "1.4.2"
val MockitoV = "2.11.0"
val MonocleV = "1.5.0-cats-M2"

val logback = "ch.qos.logback" % "logback-classic" % "1.2.+"

val cats1 = "org.typelevel" %% "cats-core" % "1.0.0-RC1"
val monix3 = "io.monix" %% "monix" % "3.0.0-M2"
val shapeless = "com.chuusai" %% "shapeless" % "2.3.+"
val monocle = "com.github.julien-truffaut" %% "monocle-core" % MonocleV
val monocleMacro = "com.github.julien-truffaut" %% "monocle-macro" % MonocleV

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
val chill = "com.twitter" %% "chill" % "0.9.2"

lazy val `fluence` = project.in(file("."))
    .settings(commons)
  .settings(
    libraryDependencies ++= Seq(
      scalatest
    )
  ).aggregate(
    `kademlia`,
    `network`,
    `storage`,
    `b-tree-client`,
    `b-tree-server`,
    `crypto`,
    `dataset`
  )

lazy val `kademlia` = project.in(file("kademlia"))
  .settings(commons)
  .settings(
    libraryDependencies ++= Seq(
      cats1,
      logback,
      scalatest,
      monix3 % Test
    )
  )

// TODO: separate API from grpc implementation
lazy val `network` = project.in(file("network"))
  .settings(commons)
  .settings(
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

// TODO: separate API from implementation for both serialization and rocksDB
lazy val `storage` = project.in(file("storage"))
  .settings(commons)
  .settings(
    libraryDependencies ++= Seq(
      rocksDb,
      typeSafeConfig,
      ficus,
      monix3,
      shapeless,
      chill,
      scalatest,
      mockito
    )
  )

lazy val `b-tree-client` = project.in(file("b-tree-client"))
  .settings(commons)
  .settings(
    libraryDependencies ++= Seq(
      monix3,
      logback,
      scalatest
    )
  ).dependsOn(`crypto`).aggregate(`crypto`)

lazy val `b-tree-server` = project.in(file("b-tree-server"))
  .settings(commons)
  .settings(
    libraryDependencies ++= Seq(
      typeSafeConfig,
      ficus,
      monix3,
      logback,
      scalatest
    )
  ).dependsOn(`storage`, `b-tree-client`).aggregate(`storage`, `b-tree-client`)

lazy val `crypto` = project.in(file("crypto"))
  .settings(commons)
  .settings(
    libraryDependencies ++= Seq(
      scalatest
    )
  )

// TODO: separate API from implementation
lazy val `dataset` = project.in(file("dataset"))
  .settings(
    scalaV,
    scalariformPrefs,
    libraryDependencies ++= Seq(
      cats1,
      scalatest
    )
  ).dependsOn(`storage`, `kademlia`)