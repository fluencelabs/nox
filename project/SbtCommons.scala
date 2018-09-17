import de.heikoseeberger.sbtheader.HeaderPlugin.autoImport.headerLicense
import de.heikoseeberger.sbtheader.License
import org.scalafmt.sbt.ScalafmtPlugin.autoImport.scalafmtOnCompile
import sbt.Keys._
import sbt._

object SbtCommons {

  val scalaV = scalaVersion := "2.12.6"

  val commons = Seq(
    scalaV,
    version                   := "0.1",
    fork in Test              := true,
    parallelExecution in Test := false,
    organizationName          := "Fluence Labs Limited",
    organizationHomepage      := Some(new URL("https://fluence.one")),
    startYear                 := Some(2018),
    licenses += ("Apache-2.0", new URL("https://www.apache.org/licenses/LICENSE-2.0.txt")),
    headerLicense := Some(License.ALv2("2018", organizationName.value)),
    resolvers += Resolver.bintrayRepo("fluencelabs", "releases"),
    scalafmtOnCompile := true,
    // see good explanation https://gist.github.com/djspiewak/7a81a395c461fd3a09a6941d4cd040f2
    scalacOptions += "-Ypartial-unification"
  )

  /* Common deps */

  val slogging = "biz.enef"        %% "slogging"    % "0.6.1"
  val cats = "org.typelevel"       %% "cats-core"   % "1.2.0"
  val catsEffect = "org.typelevel" %% "cats-effect" % "1.0.0-RC2"
  // functional wrapper around 'lightbend/config'
  val pureConfig = "com.github.pureconfig" %% "pureconfig"      % "0.9.1"
  val cryptoHashing = "one.fluence"        %% "crypto-hashsign" % "0.0.2"
  val cryptoCipher = "one.fluence"         %% "crypto-cipher"   % "0.0.2"
  val cryptoKeyStore = "one.fluence"       %% "crypto-keystore" % "0.0.2"

  val sttpVersion = "1.3.0"
  val sttp = "com.softwaremill.sttp"            %% "core"                           % sttpVersion
  val sttpCirce = "com.softwaremill.sttp"       %% "circe"                          % "1.3.0"
  val sttpCatsBackend = "com.softwaremill.sttp" %% "async-http-client-backend-cats" % "1.3.0"

  val circeVersion = "0.9.3"
  val circeCore = "io.circe"          %% "circe-core"           % circeVersion
  val circeGeneric = "io.circe"       %% "circe-generic"        % circeVersion
  val circeGenericExtras = "io.circe" %% "circe-generic-extras" % circeVersion
  val circeParser = "io.circe"        %% "circe-parser"         % circeVersion

  val scodecBits = "org.scodec" %% "scodec-bits" % "1.1.6"
  val scodecCore = "org.scodec" %% "scodec-core" % "1.10.3"
  val web3jCrypto = "org.web3j" % "crypto"       % "3.5.0"

  /* Test deps*/

  val scalaTest = "org.scalatest" %% "scalatest"   % "3.0.5"  % Test
  val mockito = "org.mockito"     % "mockito-core" % "2.21.0" % Test
  val utest = "com.lihaoyi" %% "utest" % "0.6.3" % Test

  /* Test settings */
  val setUTestFramework =
    testFrameworks += new TestFramework("utest.runner.Framework")
}
