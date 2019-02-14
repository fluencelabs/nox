import de.heikoseeberger.sbtheader.HeaderPlugin.autoImport.headerLicense
import de.heikoseeberger.sbtheader.License
import org.scalafmt.sbt.ScalafmtPlugin.autoImport.scalafmtOnCompile
import sbt.Keys._
import sbt.{Def, _}
import sbtdocker.DockerPlugin.autoImport.docker
import sys.process._

object SbtCommons {

  val scalaV = scalaVersion := "2.12.8"

  val commons = Seq(
    scalaV,
    version                              := "0.1",
    fork in Test                         := true,
    parallelExecution in Test            := false,
    fork in IntegrationTest              := true,
    parallelExecution in IntegrationTest := false,
    organizationName                     := "Fluence Labs Limited",
    organizationHomepage                 := Some(new URL("https://fluence.one")),
    startYear                            := Some(2018),
    licenses += ("Apache-2.0", new URL("https://www.apache.org/licenses/LICENSE-2.0.txt")),
    headerLicense := Some(License.ALv2("2018", organizationName.value)),
    resolvers += Resolver.bintrayRepo("fluencelabs", "releases"),
    scalafmtOnCompile := true,
    // see good explanation https://gist.github.com/djspiewak/7a81a395c461fd3a09a6941d4cd040f2
    scalacOptions ++= Seq("-Ypartial-unification", "-deprecation")
  )

  val kindProjector = Seq(
    resolvers += Resolver.sonatypeRepo("releases"),
    addCompilerPlugin("org.spire-math" %% "kind-projector" % "0.9.9")
  )

  def rustVmExample(exampleName: String): Seq[Def.Setting[_]] =
    Seq(
      publishArtifact := false,
      test            := (test in Test).dependsOn(compile).value,
      compile := (compile in Compile)
        .dependsOn(Def.task {
          val log = streams.value.log
          log.info(s"Compiling $exampleName.rs to $exampleName.wasm")

          val projectRoot = file("").getAbsolutePath
          val exampleFolder = s"$projectRoot/vm/examples/$exampleName"
          val compileCmd = s"cargo +nightly-2019-01-08 build --manifest-path $exampleFolder/Cargo.toml " +
            s"--target wasm32-unknown-unknown --release"

          assert((compileCmd !) == 0, "Rust to Wasm compilation failed")
        })
        .value
    )

  def buildContractBeforeDocker(): Seq[Def.Setting[_]] =
    Seq(
      docker in docker := (docker in docker)
        .dependsOn(Def.task {
          val log = streams.value.log
          log.info(s"Generating java wrapper for smart contracct")

          val projectRoot = file("").getAbsolutePath
          val bootstrapFolder = file(s"$projectRoot/bootstrap")
          val generateCmd = "npm run generate-all"
          log.info(s"running $generateCmd in $bootstrapFolder")

          val exitCode = Process(generateCmd, cwd = bootstrapFolder).!
          assert(
            exitCode == 0,
            "Generating java wrapper or contract compilation failed"
          )
        })
        .value
    )

  /* Common deps */
  
  val asmble = "com.github.cretz.asmble" % "asmble-compiler" % "0.4.2-fl"
  
  val slogging = "biz.enef"        %% "slogging"    % "0.6.1"
  val cats = "org.typelevel"       %% "cats-core"   % "1.5.0"
  val catsEffect = "org.typelevel" %% "cats-effect" % "1.2.0"

  val fs2Version = "1.0.2"
  val fs2 = "co.fs2"   %% "fs2-core"             % fs2Version
  val fs2rx = "co.fs2" %% "fs2-reactive-streams" % fs2Version
  val fs2io = "co.fs2" %% "fs2-io"               % fs2Version
  // functional wrapper around 'lightbend/config'
  val pureConfig = "com.github.pureconfig" %% "pureconfig"      % "0.10.1"
  val cryptoHashing = "one.fluence"        %% "crypto-hashsign" % "0.0.3"
  val cryptoCipher = "one.fluence"         %% "crypto-cipher"   % "0.0.3"
  val cryptoKeyStore = "one.fluence"       %% "crypto-keystore" % "0.0.3"

  val sttpVersion = "1.5.4"
  val sttp = "com.softwaremill.sttp"            %% "core"                           % sttpVersion
  val sttpCirce = "com.softwaremill.sttp"       %% "circe"                          % sttpVersion
  val sttpCatsBackend = "com.softwaremill.sttp" %% "async-http-client-backend-cats" % sttpVersion

  val http4sVersion = "0.20.0-M5"
  val http4sDsl = "org.http4s"    %% "http4s-dsl"          % http4sVersion
  val http4sServer = "org.http4s" %% "http4s-blaze-server" % http4sVersion
  val http4sCirce = "org.http4s"  %% "http4s-circe"        % http4sVersion

  val circeVersion = "0.10.0"
  val circeCore = "io.circe"          %% "circe-core"           % circeVersion
  val circeGeneric = "io.circe"       %% "circe-generic"        % circeVersion
  val circeGenericExtras = "io.circe" %% "circe-generic-extras" % circeVersion
  val circeParser = "io.circe"        %% "circe-parser"         % circeVersion

  val scodecBits = "org.scodec" %% "scodec-bits" % "1.1.6"
  val scodecCore = "org.scodec" %% "scodec-core" % "1.10.3"

  val web3jVersion = "4.1.0"
  val web3jCrypto = "org.web3j" % "crypto" % web3jVersion
  val web3jCore = "org.web3j"   % "core"   % web3jVersion

  val prometheusClientVersion = "0.5.0"
  val prometheusClient = "io.prometheus"        % "simpleclient"         % prometheusClientVersion
  val prometheusClientJetty = "io.prometheus"   % "simpleclient_jetty"   % prometheusClientVersion
  val prometheusClientServlet = "io.prometheus" % "simpleclient_servlet" % prometheusClientVersion

  val toml = "com.electronwill.night-config" % "toml" % "3.4.2"

  /* Test deps*/

  val scalaTest = "org.scalatest"            %% "scalatest"   % "3.0.5"  % Test
  val scalaIntegrationTest = "org.scalatest" %% "scalatest"   % "3.0.5"  % IntegrationTest
  val mockito = "org.mockito"                % "mockito-core" % "2.21.0" % Test
}
