import com.typesafe.sbt.SbtScalariform.autoImport.scalariformPreferences
import de.heikoseeberger.sbtheader.HeaderPlugin.autoImport.headerLicense
import de.heikoseeberger.sbtheader.License
import sbt.Keys._
import sbt._
import sbtprotoc.ProtocPlugin.autoImport.PB

import scalariform.formatter.preferences._

object SbtCommons {

  val scalariformPrefs = scalariformPreferences := scalariformPreferences.value
    .setPreference(AlignSingleLineCaseStatements, true)
    .setPreference(DoubleIndentConstructorArguments, true)
    .setPreference(PreserveSpaceBeforeArguments, true)
    .setPreference(RewriteArrowSymbols, true)
    .setPreference(DoubleIndentConstructorArguments, true)
    .setPreference(DanglingCloseParenthesis, Preserve)
    .setPreference(SpaceBeforeContextColon, true)
    .setPreference(NewlineAtEndOfFile, true)

  val scalaV = scalaVersion := "2.12.4"

  val commons = Seq(
    scalaV,
    scalariformPrefs,
    version := "0.1",
    fork in test := true,
    parallelExecution in Test := false,
    organizationName := "Fluence Labs Limited",
    organizationHomepage := Some(new URL("https://fluence.ai")),
    startYear := Some(2017),
    licenses += ("AGPL-3.0", new URL("http://www.gnu.org/licenses/agpl-3.0.en.html")),
    headerLicense := Some(License.AGPLv3("2017", organizationName.value))
  )

  val RocksDbV = "5.9.2"
  val TypeSafeConfV = "1.3.2"
  val FicusV = "1.4.3"
  val MockitoV = "2.13.0"
  val MonocleV = "1.5.0-cats"
  val CirceV = "0.9.1"

  val logback = "ch.qos.logback" % "logback-classic" % "1.2.+"

  val cats1 = "org.typelevel" %% "cats-core" % "1.0.0-RC2"
  val monix3 = "io.monix" %% "monix" % "3.0.0-M3"
  val shapeless = "com.chuusai" %% "shapeless" % "2.3.+"
  val monocle = "com.github.julien-truffaut" %% "monocle-core" % MonocleV
  val monocleMacro = "com.github.julien-truffaut" %% "monocle-macro" % MonocleV
  val scodecBits = "org.scodec" %% "scodec-bits" % "1.1.5"
  val bouncyCastle = "org.bouncycastle" % "bcprov-jdk15on" % "1.59"

  val circe: Seq[ModuleID] = Seq(
    "io.circe" %% "circe-core",
    "io.circe" %% "circe-generic",
    "io.circe" %% "circe-parser"
  ).map(_ % CirceV)

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
}
