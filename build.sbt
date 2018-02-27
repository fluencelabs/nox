import SbtCommons._
import sbtcrossproject.crossProject
import org.scalajs.sbtplugin.ScalaJSPlugin.autoImport._

name := "fluence"

scalacOptions in Compile ++= Seq("-Ypartial-unification", "-Xdisable-assertions")

javaOptions in Test ++= Seq("-ea")

commons

enablePlugins(AutomateHeaderPlugin)

lazy val `co-fail` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "org.typelevel" %%% "cats-core" % Cats1V,
      "com.chuusai" %%% "shapeless" % ShapelessV,
      "org.scalatest" %%% "scalatest" % ScalatestV % Test
    )
  )

lazy val `co-fail-jvm` = `co-fail`.jvm

lazy val `co-fail-js` = `co-fail`.js
  .settings(
    fork in Test := false
  )

lazy val `codec-core` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("codec/core"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "org.typelevel" %%% "cats-core" % Cats1V,
      "org.scodec" %%% "scodec-bits" % ScodecBitsV,
    )
  )

lazy val `codec-core-jvm` = `codec-core`.jvm

lazy val `codec-core-js` = `codec-core`.js
  .settings(
    fork in Test := false
  )

lazy val `codec-kryo` = project.in(file("codec/kryo"))
  .dependsOn(`codec-core-jvm`)

lazy val `kademlia-core` = project.in(file("kademlia/core"))
  .dependsOn(`kademlia-protocol`)

lazy val `kademlia-protocol` = project.in(file("kademlia/protocol"))
  .dependsOn(`codec-core-jvm`, `crypto-jvm`)

lazy val `kademlia-testkit` = project.in(file("kademlia/testkit"))
  .dependsOn(`kademlia-core`)

lazy val `kademlia-grpc` = project.in(file("kademlia/grpc"))
  .dependsOn(`transport-grpc`, `kademlia-protocol`, `codec-core-jvm`, `kademlia-testkit` % Test)

lazy val `kademlia-monix` = project.in(file("kademlia/monix"))
  .dependsOn(`kademlia-core`)

lazy val `transport-grpc` = project.in(file("transport/grpc"))
  .dependsOn(`transport-core`, `codec-core-jvm`)

lazy val `transport-core` = project.in(file("transport/core"))
  .dependsOn(`kademlia-protocol`)

lazy val `storage` = project.in(file("storage/core"))
  .dependsOn(`codec-core-jvm`)

lazy val `storage-rocksdb` = project.in(file("storage/rocksdb"))
  .dependsOn(`storage`)

lazy val `b-tree-client` = project.in(file("b-tree/client"))
  .dependsOn(`b-tree-common`, `b-tree-protocol`)

lazy val `b-tree-common` = project.in(file("b-tree/common"))
  .dependsOn(`crypto-jvm`)

lazy val `b-tree-protocol` = project.in(file("b-tree/protocol"))
  .dependsOn(`b-tree-common`)

lazy val `b-tree-server` = project.in(file("b-tree/server"))
  .dependsOn(`storage-rocksdb`, `codec-kryo`, `b-tree-common`, `b-tree-protocol`, `b-tree-client` % "compile->test")

lazy val `crypto` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .dependsOn(`codec-core`)
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "org.typelevel" %%% "cats-core" % Cats1V,
      "org.scodec" %%% "scodec-bits" % ScodecBitsV,
      "io.circe" %%% "circe-core" % CirceV,
      "io.circe" %%% "circe-parser" % CirceV,
      "biz.enef" %%% "slogging" % SloggingV,
      "org.scalatest" %%% "scalatest" % ScalatestV % Test
    )
  )

lazy val `crypto-jvm` = `crypto`.jvm.settings(
  libraryDependencies ++= Seq(
    //JVM-specific provider for cryptography
    bouncyCastle
  )
)

lazy val `crypto-js` = `crypto`.js
  .enablePlugins(ScalaJSBundlerPlugin)
  .settings(
    npmDependencies in Compile ++= Seq(
      "elliptic" -> "6.4.0",
      "crypto-js" -> "3.1.9-1"
    ),
    scalaJSModuleKind := ModuleKind.CommonJSModule,
    //all JavaScript dependencies will be concatenated to a single file *-jsdeps.js
    skip in packageJSDependencies := false,
    fork in Test := false,
    // This is an application with a main method
    scalaJSUseMainModuleInitializer := true
  )

lazy val `client` = project.in(file("client"))
  .dependsOn(`transport-grpc`, `kademlia-grpc`, `dataset-grpc`, `transport-core`, `kademlia-monix`, `dataset-protocol`)

lazy val `dataset-node` = project.in(file("dataset/node"))
  .dependsOn(`storage`, `kademlia-core`, `b-tree-server`, `kademlia-testkit` % Test, `dataset-client`, `b-tree-client`,
`dataset-client` % "compile->test")

lazy val `dataset-protocol` = project.in(file("dataset/protocol"))
  .dependsOn(`kademlia-protocol`, `b-tree-protocol`)

lazy val `dataset-grpc` = project.in(file("dataset/grpc"))
  .dependsOn(`dataset-client`, `transport-grpc`)

lazy val `dataset-client` = project.in(file("dataset/client"))
  .dependsOn(`dataset-protocol`, `crypto-jvm`, `b-tree-client`, `kademlia-core`)

lazy val `node` = project
  .dependsOn(`kademlia-grpc`, `kademlia-monix`, `dataset-node`, `dataset-grpc`, `client`)