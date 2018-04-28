import de.heikoseeberger.sbtheader.HeaderPlugin.autoImport.headerLicense
import de.heikoseeberger.sbtheader.License
import sbt.Keys._
import sbt._
import sbtprotoc.ProtocPlugin.autoImport.PB
import org.scalajs.sbtplugin.ScalaJSPlugin.autoImport._

object SbtCommons {

  val scalaV = scalaVersion := "2.12.5"

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
    resolvers += Resolver.bintrayRepo("fluencelabs", "releases")
  )

  val CodecV = "0.0.1"

  val Cats1V = "1.1.0"
  val CatsEffectV = "1.0.0-RC"
  val ScodecBitsV = "1.1.5"
  val ScodecCoreV = "1.10.3"
  val RocksDbV = "5.9.2"
  val TypeSafeConfV = "1.3.2"
  val FicusV = "1.4.3"
  val MockitoV = "2.13.0"
  val MonocleV = "1.5.0-cats"
  val CirceV = "0.9.3"
  val AirflameV = "0.41"
  val ScalatestV = "3.0.+"
  val ScalacheckV = "1.13.4"
  val SloggingV = "0.6.1"
  val ShapelessV = "2.3.+"
  val MonixV = "3.0.0-RC1"
  val FastparseV = "1.0.0"
  val jsJavaTimeV = "0.2.3"
  val Http4sV = "0.18.5"
  val fs2ReactiveStreamsV = "0.5.1"

  val slogging = "biz.enef"      %% "slogging"       % SloggingV
  val sloggingSlf4j = "biz.enef" %% "slogging-slf4j" % SloggingV

  val npmProtobufV = "3.5.0"
  val npmTypesProtobufV = "3.2.7"
  val npmGrpcWebClientV = "0.5.0"
  val npmTsProtocGenV = "0.5.2"

  val npmProtobuf = "google-protobuf" -> npmProtobufV
  val npmTypesProtobuf = "@types/google-protobuf" -> npmTypesProtobufV
  val npmGrpcWebClient = "grpc-web-client" -> npmGrpcWebClientV
  val npmTsProtocGen = "ts-protoc-gen" -> npmTsProtocGenV

  val fluenceCodec = "one.fluence"                 %% "codec-core"           % CodecV
  val cats1 = "org.typelevel"                      %% "cats-core"            % Cats1V
  val catsFree = "org.typelevel"                   %% "cats-free"            % Cats1V
  val catsEffect = "org.typelevel"                 %% "cats-effect"          % CatsEffectV
  val monix3 = "io.monix"                          %% "monix"                % MonixV
  val shapeless = "com.chuusai"                    %% "shapeless"            % ShapelessV
  val monocle = "com.github.julien-truffaut"       %% "monocle-core"         % MonocleV
  val monocleMacro = "com.github.julien-truffaut"  %% "monocle-macro"        % MonocleV
  val scodecBits = "org.scodec"                    %% "scodec-bits"          % ScodecBitsV
  val scodecCore = "org.scodec"                    %% "scodec-core"          % ScodecCoreV
  val bouncyCastle = "org.bouncycastle"            % "bcprov-jdk15on"        % "1.59"
  val http4sDsl = "org.http4s"                     %% "http4s-dsl"           % Http4sV
  val http4sBlazeServer = "org.http4s"             %% "http4s-blaze-server"  % Http4sV
  val fs2ReactiveStreams = "com.github.zainab-ali" %% "fs2-reactive-streams" % fs2ReactiveStreamsV

  val circeCore = "io.circe"   %% "circe-core"   % CirceV
  val circeParser = "io.circe" %% "circe-parser" % CirceV

  val fastParse = "com.lihaoyi"  %% "fastparse"   % FastparseV
  val scopt = "com.github.scopt" %% "scopt"       % "3.7.0"
  val jline = "org.jline"        % "jline-reader" % "3.6.1"

  val rocksDb = "org.rocksdb"         % "rocksdbjni" % RocksDbV
  val typeSafeConfig = "com.typesafe" % "config"     % TypeSafeConfV
  val ficus = "com.iheart"            %% "ficus"     % FicusV

  val mockito = "org.mockito"        % "mockito-core" % MockitoV % Test
  val scalatestKit = "org.scalatest" %% "scalatest" % ScalatestV
  val scalacheck = "org.scalacheck"  %% "scalacheck" % ScalacheckV % Test
  val scalatest = scalatestKit       % Test

  val kindProjector = addCompilerPlugin("org.spire-math" %% "kind-projector" % "0.9.6")

  val protobuf = Seq(
    PB.targets in Compile := Seq(
      scalapb.gen(flatPackage = true) -> (sourceManaged in Compile).value
    ),
    libraryDependencies ++= Seq(
      "com.thesamet.scalapb" %% "scalapb-runtime" % scalapb.compiler.Version.scalapbVersion % "protobuf"
    )
  )

  val grpc = protobuf ++ Seq(
    libraryDependencies ++= Seq(
      "io.grpc"              % "grpc-netty"            % scalapb.compiler.Version.grpcJavaVersion,
      "com.thesamet.scalapb" %% "scalapb-runtime-grpc" % scalapb.compiler.Version.scalapbVersion
    )
  )
  val chill = "com.twitter" %% "chill" % "0.9.2"
}
