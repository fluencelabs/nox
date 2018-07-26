import de.heikoseeberger.sbtheader.HeaderPlugin.autoImport.headerLicense
import de.heikoseeberger.sbtheader.License
import sbt.Keys._
import sbt._
import org.scalafmt.sbt.ScalafmtPlugin.autoImport.scalafmtOnCompile

object SbtCommons {

  val scalaV = scalaVersion := "2.12.6"

  val commons = Seq(
    scalaV,
    version := "0.1",
    fork in Test := true,
    parallelExecution in Test := false,
    organizationName := "Fluence Labs Limited",
    organizationHomepage := Some(new URL("https://fluence.one")),
    startYear := Some(2017),
    licenses += ("AGPL-3.0", new URL("http://www.gnu.org/licenses/agpl-3.0.en.html")),
    headerLicense := Some(License.AGPLv3("2017", organizationName.value)),
    resolvers += Resolver.bintrayRepo("fluencelabs", "releases"),
    scalafmtOnCompile := true
  )
}
