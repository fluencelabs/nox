import SbtCommons._
import org.scalajs.sbtplugin.ScalaJSPlugin.autoImport._
import sbtcrossproject.crossProject

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
  .enablePlugins(AutomateHeaderPlugin)

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
  .jsSettings(
    fork in Test := false
  )
  .enablePlugins(AutomateHeaderPlugin)

lazy val `codec-core-jvm` = `codec-core`.jvm

lazy val `codec-core-js` = `codec-core`.js

lazy val `codec-kryo` = project.in(file("codec/kryo"))
  .dependsOn(`codec-core-jvm`)

lazy val `kademlia-protocol` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("kademlia/protocol"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "io.circe" %%% "circe-core" % CirceV,
      "io.circe" %%% "circe-parser" % CirceV,
      "org.typelevel" %%% "cats-core" % Cats1V,
      "org.scalatest" %%% "scalatest" % ScalatestV % Test
    )
  )
  .jsSettings(
    fork in Test := false,
    scalaJSModuleKind := ModuleKind.CommonJSModule
  )
  .dependsOn(`codec-core`, `crypto`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `kademlia-protocol-js` = `kademlia-protocol`.js
  .enablePlugins(ScalaJSBundlerPlugin)

lazy val `kademlia-protocol-jvm` = `kademlia-protocol`.jvm

lazy val `kademlia-core` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("kademlia/core"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "biz.enef" %%% "slogging" % SloggingV,
      "io.monix" %%% "monix" % MonixV % Test,
      "org.scalatest" %%% "scalatest" % ScalatestV % Test
    )
  )
  .jsSettings(
    libraryDependencies ++= Seq(
      "org.scala-js" %%% "scalajs-java-time" % "0.2.3"
    ),
    fork in Test := false,
    scalaJSModuleKind := ModuleKind.CommonJSModule
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`kademlia-protocol`)

lazy val `kademlia-core-js` = `kademlia-core`.js
lazy val `kademlia-core-jvm` = `kademlia-core`.jvm

lazy val `kademlia-testkit` = project.in(file("kademlia/testkit"))
  .dependsOn(`kademlia-core-jvm`)

lazy val `kademlia-grpc` = project.in(file("kademlia/grpc"))
  .dependsOn(`transport-grpc`, `kademlia-protocol-jvm`, `codec-core-jvm`, `kademlia-testkit` % Test)

lazy val `kademlia-monix` =
  crossProject(JVMPlatform, JSPlatform)
    .withoutSuffixFor(JVMPlatform)
    .crossType(FluenceCrossType)
    .in(file("kademlia/monix"))
    .settings(
      commons,
      libraryDependencies ++= Seq(
        "io.monix" %%% "monix" % MonixV,
        "org.scalatest" %%% "scalatest" % ScalatestV % Test
      )
    )
    .jsSettings(
      fork in Test := false,
      scalaJSModuleKind := ModuleKind.CommonJSModule
    )
    .enablePlugins(AutomateHeaderPlugin)
    .dependsOn(`kademlia-core`)

lazy val `kademlia-monix-js` = `kademlia-monix`.js
lazy val `kademlia-monix-jvm` = `kademlia-monix`.jvm

lazy val `transport-grpc` = project.in(file("transport/grpc"))
  .dependsOn(`transport-core-jvm`, `codec-core-jvm`)

lazy val `transport-core` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("transport/core"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "org.typelevel" %%% "cats-core" % Cats1V,
      "com.chuusai" %%% "shapeless" % ShapelessV,
      "biz.enef" %%% "slogging" % SloggingV,
      "org.typelevel" %%% "cats-effect" % CatsEffectV
    )
  )
  .jvmSettings(
    libraryDependencies ++= Seq(
      "org.bitlet" % "weupnp" % "0.1.+"
    )
  )
  .jsSettings(
    fork in Test := false,
    scalaJSModuleKind := ModuleKind.CommonJSModule
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`kademlia-protocol`)

lazy val `transport-core-js` = `transport-core`.js
lazy val `transport-core-jvm` = `transport-core`.jvm

lazy val `storage-core` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("storage/core"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "org.scalatest" %%% "scalatest" % ScalatestV % Test,
      "io.monix" %%% "monix" % MonixV % Test
    )
  ).jsSettings(
    fork in Test := false,
    scalaJSModuleKind := ModuleKind.CommonJSModule
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`codec-core`)

lazy val `storage-core-jvm` = `storage-core`.jvm
lazy val `storage-core-js` = `storage-core`.js

lazy val `storage-rocksdb` = project.in(file("storage/rocksdb"))
  .dependsOn(`storage-core-jvm`)

// core entities for all b-tree modules
lazy val `b-tree-core` = project.in(file("b-tree/core"))
  .dependsOn(`codec-core-jvm`) // todo change to crossProject codec-core

lazy val `b-tree-protocol` = project.in(file("b-tree/protocol"))
  .dependsOn(`b-tree-core`)

// common logic for client and server
lazy val `b-tree-common` = project.in(file("b-tree/common"))
  .dependsOn(`b-tree-core`, `crypto-jvm`)

lazy val `b-tree-client` = project.in(file("b-tree/client"))
  .dependsOn(`b-tree-common`, `b-tree-protocol`)

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
  .jvmSettings(
    libraryDependencies ++= Seq(
      //JVM-specific provider for cryptography
      bouncyCastle
    )
  )
  .jsSettings(
    npmDependencies in Compile ++= Seq(
      "elliptic" -> "6.4.0",
      "crypto-js" -> "3.1.9-1"
    ),
    scalaJSModuleKind := ModuleKind.CommonJSModule,
    //all JavaScript dependencies will be concatenated to a single file *-jsdeps.js
    skip in packageJSDependencies := false,
    fork in Test := false
  )
  .enablePlugins(AutomateHeaderPlugin)

lazy val `crypto-jvm` = `crypto`.jvm

lazy val `crypto-js` = `crypto`.js
  .enablePlugins(ScalaJSBundlerPlugin)

lazy val `client` = project.in(file("client"))
  .dependsOn(`transport-grpc`, `kademlia-grpc`, `dataset-grpc`, `transport-core-jvm`, `kademlia-monix-jvm`, `dataset-protocol`, `contract-protocol-jvm`)

lazy val `dataset-node` = project.in(file("dataset/node"))
  .dependsOn(`storage-core-jvm`, `kademlia-core-jvm`, `b-tree-server`, `kademlia-testkit` % Test, `dataset-client`, `b-tree-client`,
    `dataset-client` % "compile->test")

lazy val `dataset-protocol` = project.in(file("dataset/protocol"))
  .dependsOn(`kademlia-protocol-jvm`, `b-tree-protocol`)

lazy val `dataset-grpc` = project.in(file("dataset/grpc"))
  .dependsOn(`dataset-client`, `transport-grpc`)

lazy val `dataset-client` = project.in(file("dataset/client"))
  .dependsOn(`dataset-protocol`, `crypto-jvm`, `b-tree-client`, `kademlia-core-jvm`, `contract-core-jvm`)

lazy val `contract-protocol` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("contract/protocol"))
  .settings(commons)
  .jsSettings(
    fork in Test := false,
    scalaJSModuleKind := ModuleKind.CommonJSModule
  ).enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`kademlia-protocol`)

lazy val `contract-protocol-js` = `contract-protocol`.js
lazy val `contract-protocol-jvm` = `contract-protocol`.jvm

lazy val `contract-core` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("contract/core"))
  .settings(commons)
  .jsSettings(
    fork in Test := false,
    scalaJSModuleKind := ModuleKind.CommonJSModule
  ).enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`contract-protocol`, `crypto`)

lazy val `contract-core-js` = `contract-core`.js
lazy val `contract-core-jvm` = `contract-core`.jvm

lazy val `node` = project
  .dependsOn(`kademlia-grpc`, `kademlia-monix-jvm`, `dataset-node`, `dataset-grpc`, `client`, `transport-core-jvm`)