import SbtCommons._
import org.scalajs.sbtplugin.ScalaJSPlugin.autoImport._
import sbtcrossproject.crossProject

name := "fluence"

scalacOptions in Compile ++= Seq("-Ypartial-unification", "-Xdisable-assertions")

javaOptions in Test ++= Seq("-ea")

commons

enablePlugins(AutomateHeaderPlugin)

lazy val `codec-core` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("codec/core"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "org.typelevel" %%% "cats-core"   % Cats1V,
      "org.scodec"    %%% "scodec-bits" % ScodecBitsV
    )
  )
  .jsSettings(
    fork in Test := false
  )
  .enablePlugins(AutomateHeaderPlugin)

lazy val `codec-core-jvm` = `codec-core`.jvm
lazy val `codec-core-js` = `codec-core`.js

lazy val `codec-kryo` = project
  .in(file("codec/kryo"))
  .settings(commons)
  .dependsOn(`codec-core-jvm`)

lazy val `codec-protobuf` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("codec/protobuf"))
  .settings(
    commons,
    protobuf
  )
  .jsSettings(
    fork in Test      := false,
    scalaJSModuleKind := ModuleKind.CommonJSModule
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`codec-core`, `kademlia-protocol`)

lazy val `codec-protobuf-jvm` = `codec-protobuf`.jvm
lazy val `codec-protobuf-js` = `codec-protobuf`.js

lazy val `kademlia-protocol` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("kademlia/protocol"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "io.circe"      %%% "circe-core"   % CirceV,
      "io.circe"      %%% "circe-parser" % CirceV,
      "org.typelevel" %%% "cats-core"    % Cats1V,
      "org.typelevel" %%% "cats-effect" % CatsEffectV,
      "org.scalatest" %%% "scalatest"    % ScalatestV % Test
    )
  )
  .jsSettings(
    fork in Test      := false,
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
      "biz.enef"      %%% "slogging"  % SloggingV,
      "io.monix"      %%% "monix"     % MonixV % Test,
      "org.scalatest" %%% "scalatest" % ScalatestV % Test
    )
  )
  .jsSettings(
    libraryDependencies ++= Seq(
      "org.scala-js" %%% "scalajs-java-time" % "0.2.3"
    ),
    fork in Test      := false,
    scalaJSModuleKind := ModuleKind.CommonJSModule
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`kademlia-protocol`)

lazy val `kademlia-core-js` = `kademlia-core`.js
lazy val `kademlia-core-jvm` = `kademlia-core`.jvm

lazy val `kademlia-testkit` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("kademlia/testkit"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "io.monix"      %%% "monix"     % MonixV,
      "org.scalatest" %%% "scalatest" % ScalatestV % Test
    )
  )
  .jsSettings(
    fork in Test      := false,
    scalaJSModuleKind := ModuleKind.CommonJSModule
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`kademlia-core`)

lazy val `kademlia-testkit-js` = `kademlia-testkit`.js
lazy val `kademlia-testkit-jvm` = `kademlia-testkit`.jvm

lazy val `kademlia-grpc` = project
  .in(file("kademlia/grpc"))
  .dependsOn(`transport-grpc`, `kademlia-protocol-jvm`, `codec-core-jvm`, `kademlia-testkit-jvm` % Test)

lazy val `kademlia-monix` =
  crossProject(JVMPlatform, JSPlatform)
    .withoutSuffixFor(JVMPlatform)
    .crossType(FluenceCrossType)
    .in(file("kademlia/monix"))
    .settings(
      commons,
      libraryDependencies ++= Seq(
        "io.monix"      %%% "monix"     % MonixV,
        "org.scalatest" %%% "scalatest" % ScalatestV % Test
      )
    )
    .jsSettings(
      fork in Test      := false,
      scalaJSModuleKind := ModuleKind.CommonJSModule
    )
    .enablePlugins(AutomateHeaderPlugin)
    .dependsOn(`kademlia-core`)

lazy val `kademlia-monix-js` = `kademlia-monix`.js
lazy val `kademlia-monix-jvm` = `kademlia-monix`.jvm

lazy val `transport-grpc` = project
  .in(file("transport/grpc"))
  .dependsOn(`transport-core-jvm`, `codec-protobuf-jvm`)

lazy val `transport-core` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("transport/core"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "org.typelevel" %%% "cats-core"   % Cats1V,
      "com.chuusai"   %%% "shapeless"   % ShapelessV,
      "biz.enef"      %%% "slogging"    % SloggingV,
      "org.typelevel" %%% "cats-effect" % CatsEffectV
    )
  )
  .jvmSettings(
    libraryDependencies ++= Seq(
      "org.bitlet" % "weupnp" % "0.1.+"
    )
  )
  .jsSettings(
    fork in Test      := false,
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
      "io.monix"      %%% "monix"     % MonixV     % Test
    )
  )
  .jsSettings(
    fork in Test      := false,
    scalaJSModuleKind := ModuleKind.CommonJSModule
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`codec-core`)

lazy val `storage-core-jvm` = `storage-core`.jvm
lazy val `storage-core-js` = `storage-core`.js

lazy val `storage-rocksdb` = project
  .in(file("storage/rocksdb"))
  .dependsOn(`storage-core-jvm`)

// core entities for all b-tree modules
lazy val `b-tree-core` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("b-tree/core"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "org.scodec"    %%% "scodec-bits" % ScodecBitsV,
      "org.typelevel" %%% "cats-core"   % Cats1V,
      "org.scalatest" %%% "scalatest"   % ScalatestV % Test
    )
  )
  .jsSettings(
    fork in Test := false
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`codec-core`)

lazy val `b-tree-core-js` = `b-tree-core`.js
lazy val `b-tree-core-jvm` = `b-tree-core`.jvm

lazy val `b-tree-protocol` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("b-tree/protocol"))
  .settings(
    commons
  )
  .jsSettings(
    fork in Test := false
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`b-tree-core`)

lazy val `b-tree-protocol-js` = `b-tree-protocol`.js
lazy val `b-tree-protocol-jvm` = `b-tree-protocol`.jvm

// common logic for client and server
lazy val `b-tree-common` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("b-tree/common"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "org.scalatest" %%% "scalatest" % ScalatestV % Test
    )
  )
  .jsSettings(
    fork in Test := false
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`b-tree-core`, `crypto`)

lazy val `b-tree-common-js` = `b-tree-common`.js
lazy val `b-tree-common-jvm` = `b-tree-common`.jvm

lazy val `b-tree-client` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("b-tree/client"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "io.monix"      %%% "monix"     % MonixV,
      "biz.enef"      %%% "slogging"  % SloggingV,
      "org.scalatest" %%% "scalatest" % ScalatestV % Test
    )
  )
  .jsSettings(
    fork in Test := false
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`b-tree-common`, `b-tree-protocol`)

lazy val `b-tree-client-js` = `b-tree-client`.js
lazy val `b-tree-client-jvm` = `b-tree-client`.jvm

lazy val `b-tree-server` = project
  .in(file("b-tree/server"))
  .dependsOn(
    `storage-rocksdb`,
    `codec-kryo`,
    `b-tree-common-jvm`,
    `b-tree-protocol-jvm`,
    `b-tree-client-jvm` % "compile->test"
  )

lazy val `crypto` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "org.typelevel" %%% "cats-core"    % Cats1V,
      "org.scodec"    %%% "scodec-bits"  % ScodecBitsV,
      "io.circe"      %%% "circe-core"   % CirceV,
      "io.circe"      %%% "circe-parser" % CirceV,
      "biz.enef"      %%% "slogging"     % SloggingV,
      "org.scalatest" %%% "scalatest"    % ScalatestV % Test
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
    fork in Test                  := false
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`codec-core`)

lazy val `crypto-jvm` = `crypto`.jvm

lazy val `crypto-js` = `crypto`.js
  .enablePlugins(ScalaJSBundlerPlugin)

lazy val `dataset-node` = project
  .in(file("dataset/node"))
  .dependsOn(`storage-core-jvm`, `kademlia-core-jvm`, `b-tree-server`, `dataset-client-jvm`, `b-tree-client-jvm`)

lazy val `dataset-protocol` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("dataset/protocol"))
  .settings(commons)
  .jsSettings(
    fork in Test := false
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`kademlia-protocol`, `b-tree-protocol`)

lazy val `dataset-protocol-jvm` = `dataset-protocol`.jvm
lazy val `dataset-protocol-js` = `dataset-protocol`.js

lazy val `dataset-grpc` = project
  .in(file("dataset/grpc"))
  .dependsOn(`dataset-client-jvm`, `transport-grpc`)

lazy val `dataset-client` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("dataset/client"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "org.typelevel" %%% "cats-core" % Cats1V
    )
  )
  .jsSettings(
    fork in Test := false
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`dataset-protocol`, `crypto`, `b-tree-client`, `kademlia-core`)

lazy val `dataset-client-js` = `dataset-client`.js
lazy val `dataset-client-jvm` = `dataset-client`.jvm

lazy val `contract-protocol` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("contract/protocol"))
  .settings(commons)
  .jsSettings(
    fork in Test := false
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`kademlia-protocol`)

lazy val `contract-protocol-js` = `contract-protocol`.js
lazy val `contract-protocol-jvm` = `contract-protocol`.jvm

lazy val `contract-core` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("contract/core"))
  .settings(commons)
  .jsSettings(
    fork in Test      := false,
    scalaJSModuleKind := ModuleKind.CommonJSModule
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`contract-protocol`, `crypto`)

lazy val `contract-core-js` = `contract-core`.js
lazy val `contract-core-jvm` = `contract-core`.jvm

lazy val `contract-client` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("contract/client"))
  .settings(commons)
  .jsSettings(
    fork in Test := false
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`contract-core`, `kademlia-core`)

lazy val `contract-client-js` = `contract-client`.js
lazy val `contract-client-jvm` = `contract-client`.jvm

lazy val `contract-node` = project
  .in(file("contract/node"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      monix3 % Test,
      scalatest
    )
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`contract-core-jvm`, `storage-core-jvm`, `contract-client-jvm` % Test, `kademlia-testkit-jvm` % Test)

lazy val `contract-grpc` = project
  .in(file("contract/grpc"))
  .settings(
    commons,
    grpc,
    libraryDependencies ++= Seq(
      scalatest
    )
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`contract-core-jvm`, `transport-grpc`)

lazy val `client-core` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("client/core"))
  .settings(commons)
  .jvmSettings(
    libraryDependencies ++= Seq(
      typeSafeConfig,
      ficus
    )
  )
  .jsSettings(
    fork in Test := false
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`kademlia-monix`, `contract-client`, `dataset-client`, `transport-core`)

lazy val `client-core-js` = `client-core`.js
lazy val `client-core-jvm` = `client-core`.jvm

lazy val `client-grpc` = project
  .in(file("client/grpc"))
  .settings(commons)
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`client-core-jvm`, `transport-grpc`, `kademlia-grpc`, `dataset-grpc`, `contract-grpc`)

lazy val `client-cli` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("client/cli"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "com.lihaoyi"   %%% "fastparse" % FastparseV,
      "org.typelevel" %%% "cats-free" % Cats1V,
      "org.scalatest" %%% "scalatest" % ScalatestV % Test
    )
  )
  .jvmSettings(
    libraryDependencies ++= Seq(
      jline,
      scopt
    )
  )
  .jsSettings(
    fork in Test := false
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`client-core`)

lazy val `client-cli-js` = `client-cli`.js
lazy val `client-cli-jvm` = `client-cli`.jvm

lazy val `client-cli-app` = project
  .in(file("client/cli-app"))
  .settings(commons)
  .dependsOn(`client-cli-jvm`, `client-grpc`)

lazy val `node-core` = project
  .in(file("node/core"))
  .settings(
    commons,
    protobuf,
    libraryDependencies ++= Seq(
      scalatest
    )
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`dataset-node`, `contract-node`, `client-core-jvm`, `codec-protobuf-jvm`)

lazy val `node-grpc` = project
  .in(file("node/grpc"))
  .dependsOn(`node-core`, `client-grpc`)

lazy val `node` = project
  .dependsOn(`node-grpc`)
