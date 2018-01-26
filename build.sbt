import de.heikoseeberger.sbtheader.License

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
  fork in test := true,
  parallelExecution in Test := false,
  organizationName := "Fluence Labs Limited",
  organizationHomepage := Some(new URL("https://fluence.ai")),
  startYear := Some(2017),
  licenses += ("AGPL-3.0", new URL("http://www.gnu.org/licenses/agpl-3.0.en.html")),
  headerLicense := Some(License.AGPLv3("2017", organizationName.value))
)

commons

enablePlugins(AutomateHeaderPlugin)

val RocksDbV = "5.9.2"
val TypeSafeConfV = "1.3.2"
val FicusV = "1.4.3"
val MockitoV = "2.13.0"
val MonocleV = "1.5.0-cats"

val logback = "ch.qos.logback" % "logback-classic" % "1.2.+"

val cats1 = "org.typelevel" %% "cats-core" % "1.0.0-RC2"
val monix3 = "io.monix" %% "monix" % "3.0.0-M3"
val shapeless = "com.chuusai" %% "shapeless" % "2.3.+"
val monocle = "com.github.julien-truffaut" %% "monocle-core" % MonocleV
val monocleMacro = "com.github.julien-truffaut" %% "monocle-macro" % MonocleV
val scodecBits = "org.scodec" %% "scodec-bits" % "1.1.5"

val rocksDb = "org.rocksdb" % "rocksdbjni" % RocksDbV
val typeSafeConfig = "com.typesafe" % "config" % TypeSafeConfV
val ficus = "com.iheart" %% "ficus" % FicusV

val mockito = "org.mockito" % "mockito-core" % MockitoV % Test
val scalatestKit = "org.scalatest" %% "scalatest" % "3.0.+"
val scalatest = scalatestKit % Test

val protobuf = Seq(
  PB.targets in Compile := Seq(
    scalapb.gen() -> (sourceManaged in Compile).value
  ),
  libraryDependencies ++= Seq(
    "com.trueaccord.scalapb" %% "scalapb-runtime" % com.trueaccord.scalapb.compiler.Version.scalapbVersion % "protobuf"
  )
)

val grpc = protobuf ++ Seq(
  libraryDependencies ++= Seq(
    "io.grpc" % "grpc-netty" % com.trueaccord.scalapb.compiler.Version.grpcJavaVersion,
    "com.trueaccord.scalapb" %% "scalapb-runtime-grpc" % com.trueaccord.scalapb.compiler.Version.scalapbVersion
  )
)
val chill = "com.twitter" %% "chill" % "0.9.2"

lazy val `fluence` = project.in(file("."))
  .settings(commons)
  .aggregate(`node`, `client`) // Should aggregate everything else transitively, and do nothing more
  .enablePlugins(AutomateHeaderPlugin)

lazy val `codec-core` = project.in(file("codec/core"))
  .settings(commons)
  .settings(
    libraryDependencies ++= Seq(
      scodecBits,
      cats1
    )
  )

lazy val `codec-kryo` = project.in(file("codec/kryo"))
  .settings(commons)
  .settings(
    libraryDependencies ++= Seq(
      chill,
      shapeless,
      scalatest
    )
  ).dependsOn(`codec-core`).aggregate(`codec-core`)

lazy val `kademlia-node` = project.in(file("kademlia/node"))
  .settings(commons)
  .settings(
    libraryDependencies ++= Seq(
      cats1,
      logback,
      scalatest,
      monix3 % Test
    )
  ).dependsOn(`kademlia-protocol`)
  .aggregate(`kademlia-protocol`)

lazy val `kademlia-protocol` = project.in(file("kademlia/protocol"))
  .settings(commons)
  .settings(
    libraryDependencies ++= Seq(
      scodecBits,
      cats1
    )
  ).dependsOn(`codec-core`, `crypto`)

lazy val `kademlia-testkit` = project.in(file("kademlia/testkit"))
  .settings(commons)
  .settings(
    libraryDependencies ++= Seq(
      scalatestKit,
      monix3
    )
  ).dependsOn(`kademlia-node`)

lazy val `kademlia-grpc` = project.in(file("kademlia/grpc"))
  .settings(commons)
  .settings(
    grpc
  ).dependsOn(`transport-grpc`, `kademlia-protocol`, `codec-core`, `kademlia-testkit` % Test)

lazy val `transport-grpc` = project.in(file("transport/grpc"))
  .settings(commons)
  .settings(
    grpc,
    libraryDependencies ++= Seq(
      monix3,
      shapeless,
      typeSafeConfig,
      ficus,
      logback,
      "org.bitlet" % "weupnp" % "0.1.+",
      scalatest
    )
  ).dependsOn(`transport-core`, `codec-core`)

lazy val `transport-core` = project.in(file("transport/core"))
  .settings(commons)
  .settings(
    libraryDependencies ++= Seq(
      cats1,
      shapeless
    )
  ).dependsOn(`kademlia-protocol`).aggregate(`kademlia-protocol`)

lazy val `storage` = project.in(file("storage/core"))
  .settings(commons)
  .settings(
    libraryDependencies ++= Seq(
      scalatest,
      monix3 % Test
    )
  ).dependsOn(`codec-core`).aggregate(`codec-core`)

lazy val `storage-rocksdb` = project.in(file("storage/rocksdb"))
  .settings(commons)
  .settings(
    libraryDependencies ++= Seq(
      rocksDb,
      typeSafeConfig,
      ficus,
      monix3,
      scalatest,
      mockito
    )
  ).dependsOn(`storage`).aggregate(`storage`)

lazy val `b-tree-client` = project.in(file("b-tree/client"))
  .settings(commons)
  .settings(
    libraryDependencies ++= Seq(
      monix3,
      logback,
      scalatest
    )
  ).dependsOn(`b-tree-common`, `b-tree-protocol`)
  .aggregate(`b-tree-common`, `b-tree-protocol`)

lazy val `b-tree-common` = project.in(file("b-tree/common"))
  .settings(commons)
  .settings(
    libraryDependencies ++= Seq(
      cats1,
      scalatest
    )
  ).dependsOn(`crypto`)

lazy val `b-tree-protocol` = project.in(file("b-tree/protocol"))
  .settings(commons)
  .dependsOn(`b-tree-common`)
  .aggregate(`b-tree-common`)

lazy val `b-tree-server` = project.in(file("b-tree/server"))
  .settings(commons)
  .settings(
    libraryDependencies ++= Seq(
      typeSafeConfig,
      ficus,
      monix3,
      logback,
      scalatest
    )
  ).dependsOn(`storage-rocksdb`, `codec-kryo`, `b-tree-common`, `b-tree-protocol`, `b-tree-client` % "compile->test")
  .aggregate(`b-tree-common`, `b-tree-protocol`)

lazy val `crypto` = project.in(file("crypto"))
  .settings(commons)
  .settings(
    libraryDependencies ++= Seq(
      cats1,
      scodecBits,
      scalatest
    )
  )

lazy val `dataset-node` = project.in(file("dataset/node"))
  .settings(commons)
  .settings(
    libraryDependencies ++= Seq(
      cats1,
      monix3 % Test,
      scalatest
    )
  ).dependsOn(`storage`, `kademlia-node`, `b-tree-server`, `kademlia-testkit` % Test, `dataset-client`)

lazy val `dataset-protocol` = project.in(file("dataset/protocol"))
  .settings(commons)
  .dependsOn(`kademlia-protocol`).aggregate(`kademlia-protocol`)

lazy val `dataset-grpc` = project.in(file("dataset/grpc"))
  .settings(commons)
  .settings(
    grpc,
    libraryDependencies ++= Seq(
      scalatest
    )
  ).dependsOn(`dataset-client`, `codec-core`, `transport-grpc`).aggregate(`dataset-protocol`)

lazy val `dataset-client` = project.in(file("dataset/client"))
  .settings(commons)
  .settings(
    libraryDependencies ++= Seq(
      scalatest
    )
  ).dependsOn(`dataset-protocol`, `crypto`, `b-tree-client`)

lazy val `node` = project.in(file("node"))
  .settings(commons)
  .settings(
    protobuf,
    libraryDependencies ++= Seq(
      scalatest
    ),
    mainClass := Some("fluence.node.NodeApp"),
    packageName in Docker := "fluencelabs/node"
  )
  .dependsOn(`transport-grpc`, `kademlia-grpc`, `kademlia-node`, `dataset-node`, `dataset-grpc`, `client`)
  .aggregate(`transport-grpc`, `kademlia-grpc`, `kademlia-node`, `dataset-node`, `dataset-grpc`, `client`)
  .enablePlugins(JavaAppPackaging, DockerPlugin)

// TODO: add enough dependencies for client-node communications
// TODO: grpc is only for JVM: transport should be more abstract
lazy val `client` = project.in(file("client"))
  .settings(commons)
  .dependsOn(`dataset-client`, `transport-grpc`, `kademlia-grpc`, `dataset-grpc`)
  .aggregate(`dataset-client`)