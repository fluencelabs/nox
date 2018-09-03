import de.heikoseeberger.sbtheader.HeaderPlugin.autoImport.headerLicense
import de.heikoseeberger.sbtheader.License
import sbt.Keys._
import sbt._
import org.scalafmt.sbt.ScalafmtPlugin.autoImport.scalafmtOnCompile

object SbtCommons {

  val scalaV = scalaVersion := "2.12.6"

  val commons = Seq(
    scalaV,
    version                   := "0.1",
    fork in Test              := true,
    parallelExecution in Test := false,
    organizationName          := "Fluence Labs Limited",
    organizationHomepage      := Some(new URL("https://fluence.one")),
    startYear                 := Some(2017),
    licenses += ("AGPL-3.0", new URL("http://www.gnu.org/licenses/agpl-3.0.en.html")),
    headerLicense := Some(License.AGPLv3("2017", organizationName.value)),
    resolvers += Resolver.bintrayRepo("fluencelabs", "releases"),
    scalafmtOnCompile := true,
    // see good explanation https://gist.github.com/djspiewak/7a81a395c461fd3a09a6941d4cd040f2
    scalacOptions += "-Ypartial-unification"
  )

  /* Common deps */
  
  val cats = "org.typelevel"       %% "cats-core"   % "1.2.0"
  val catsEffect = "org.typelevel" %% "cats-effect" % "1.0.0-RC2"
  // functional wrapper around 'lightbend/config'
  val pureConfig = "com.github.pureconfig" %% "pureconfig"      % "0.9.1"
  val cryptoHashing = "one.fluence"        %% "crypto-hashsign" % "0.0.1"

  /* Test deps*/

  val scalaTest = "org.scalatest" %% "scalatest"    % "3.0.5"  % Test
  val mockito = "org.mockito"     % "mockito-core" % "2.21.0" % Test

}
