import com.typesafe.sbt.SbtScalariform.ScalariformKeys
import scalariform.formatter.preferences._

val scalariformPrefs =
  ScalariformKeys.preferences := ScalariformKeys.preferences.value
    .setPreference(AlignSingleLineCaseStatements, true)
    .setPreference(DoubleIndentConstructorArguments, true)
    .setPreference(DanglingCloseParenthesis, Preserve)

Seq(scalariformPrefs)

//enablePlugins(JavaServerAppPackaging)



name := "node"

version := "0.1"

val scalaV = scalaVersion := "2.12.3"

scalaV

val FreesV = "0.3.1"
val MonixV = "2.3.0"

val scalatest = "org.scalatest" %% "scalatest" % "3.0.2" % Test
val frees = "io.frees" %% "freestyle" % FreesV
val monix = "io.monix" %% "monix" % MonixV
val monixCats = "io.monix" %% "monix-cats" % MonixV
val cats = "org.typelevel" %% "cats" % "0.9.0"
val cats1 = "org.typelevel" %% "cats-core" % "1.0.0-MF"

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