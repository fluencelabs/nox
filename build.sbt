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
val RocksDbV = "5.8.0"
val TypeSafeConfV = "1.3.2"
val FicusV = "1.4.2"
val MockitoV = "2.11.0"

val scalatest = "org.scalatest" %% "scalatest" % "3.0.2" % Test
val frees = "io.frees" %% "freestyle" % FreesV
val monix = "io.monix" %% "monix" % MonixV
val monixCats = "io.monix" %% "monix-cats" % MonixV
val cats = "org.typelevel" %% "cats" % "0.9.0"

val cats1 = "org.typelevel" %% "cats-core" % "1.0.0-MF"
val monix3 = "io.monix" %% "monix" % "3.0.0-M1"

val rocksDb = "org.rocksdb" % "rocksdbjni" % RocksDbV
val typeSafeConfig = "com.typesafe" % "config" % TypeSafeConfV
val ficus = "com.iheart" %% "ficus" % FicusV
val mockito = "org.mockito" % "mockito-core" % MockitoV % Test

val paradise = addCompilerPlugin("org.scalameta" % "paradise" % "3.0.0-M10" cross CrossVersion.full)

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
  ).aggregate(
    `hack`,
    `kademlia`,
    `network`,
    `storage`
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
      scalatest
    )
  )

lazy val `network` = project.in(file("network"))
  .settings(
    scalaV,
    scalariformPrefs,
    libraryDependencies ++= Seq(
      monix3,
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