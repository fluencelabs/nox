import SbtCommons._
import org.scalajs.sbtplugin.ScalaJSPlugin.autoImport._
import sbtcrossproject.crossProject

name := "fluence"

scalacOptions in Compile ++= Seq("-Ypartial-unification", "-Xdisable-assertions")

javaOptions in Test ++= Seq("-ea")

commons

enablePlugins(AutomateHeaderPlugin)

lazy val `kademlia-protocol` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("kademlia/protocol"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "org.typelevel" %%% "cats-core"   % Cats1V,
      "org.typelevel" %%% "cats-effect" % CatsEffectV,
      "one.fluence"   %%% "codec-circe" % CodecV,
      "one.fluence"   %%% "codec-bits"  % CodecV,
      "org.scalatest" %%% "scalatest"   % ScalatestV % Test
    )
  )
  .jsSettings(
    scalaJSModuleKind := ModuleKind.CommonJSModule,
    fork in Test      := false
  )
  .dependsOn(`crypto-hashsign`)
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
      "org.scala-js" %%% "scalajs-java-time" % jsJavaTimeV
    ),
    fork in Test              := false,
    scalaJSModuleKind in Test := ModuleKind.CommonJSModule
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
    fork in Test              := false,
    scalaJSModuleKind in Test := ModuleKind.CommonJSModule
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`kademlia-core`)

lazy val `kademlia-testkit-js` = `kademlia-testkit`.js
lazy val `kademlia-testkit-jvm` = `kademlia-testkit`.jvm

lazy val `kademlia-protobuf` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("kademlia/protobuf"))
  .settings(
    commons,
    PB.targets in Compile := Seq(
      scalapb.gen(flatPackage = true) -> (sourceManaged in Compile).value
    ),
    libraryDependencies ++= Seq(
      "com.thesamet.scalapb" %%% "scalapb-runtime" % scalapb.compiler.Version.scalapbVersion,
      "com.thesamet.scalapb" %%% "scalapb-runtime" % scalapb.compiler.Version.scalapbVersion % "protobuf"
    ),
    PB.protoSources in Compile := Seq(file("kademlia/protobuf/src/main/protobuf"))
  )
  .jsSettings(
    fork in Test := false
  )
  .enablePlugins(AutomateHeaderPlugin)

lazy val `kademlia-protobuf-js` = `kademlia-protobuf`.js
lazy val `kademlia-protobuf-jvm` = `kademlia-protobuf`.jvm

lazy val `kademlia-grpc` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("kademlia/grpc"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "one.fluence" %%% "codec-core" % CodecV
    ),
    PB.protoSources in Compile := Seq(file("kademlia/grpc/src/main/protobuf"))
  )
  .jvmSettings(
    grpc
  )
  .jsSettings(
    libraryDependencies ++= Seq(
      "org.scala-js" %%% "scalajs-java-time" % jsJavaTimeV
    ),
    workbenchStartMode := WorkbenchStartModes.Manual,
    //all JavaScript dependencies will be concatenated to a single file *-jsdeps.js
    skip in packageJSDependencies   := false,
    fork in Test                    := false,
    scalaJSUseMainModuleInitializer := true
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`kademlia-protobuf`, `transport-grpc`, `kademlia-protocol`, `kademlia-testkit` % Test)

lazy val `kademlia-grpc-js` = `kademlia-grpc`.js
  .enablePlugins(WorkbenchPlugin)
  .enablePlugins(ScalaJSBundlerPlugin)
  .dependsOn(`transport-websocket-js`)

lazy val `kademlia-grpc-jvm` = `kademlia-grpc`.jvm

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
      fork in Test              := false,
      scalaJSModuleKind in Test := ModuleKind.CommonJSModule
    )
    .enablePlugins(AutomateHeaderPlugin)
    .dependsOn(`kademlia-core`)

lazy val `kademlia-monix-js` = `kademlia-monix`.js
lazy val `kademlia-monix-jvm` = `kademlia-monix`.jvm

// Default Kademlia bundle and integration tests
lazy val `kademlia` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "org.scalatest" %%% "scalatest" % ScalatestV % Test
    )
  )
  .jsSettings(
    fork in Test              := false,
    scalaJSModuleKind in Test := ModuleKind.CommonJSModule
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`kademlia-monix`, `kademlia-grpc`, `kademlia-testkit` % Test)

lazy val `kademlia-js` = `kademlia`.js
lazy val `kademlia-jvm` = `kademlia`.jvm

lazy val `transport-grpc` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("transport/grpc"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "com.chuusai"   %%% "shapeless"      % ShapelessV,
      "biz.enef"      %%% "slogging"       % SloggingV,
      "one.fluence"   %%% "codec-protobuf" % CodecV,
      "org.scalatest" %%% "scalatest"      % ScalatestV % Test
    )
  )
  .jvmSettings(
    grpc,
    libraryDependencies ++= Seq(
      typeSafeConfig,
      ficus
    )
  )
  .jsSettings(
    fork in Test              := false,
    scalaJSModuleKind in Test := ModuleKind.CommonJSModule
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`transport-core`, `kademlia-protocol`)

lazy val `transport-grpc-js` = `transport-grpc`.js
lazy val `transport-grpc-jvm` = `transport-grpc`.jvm

lazy val `websocket-protobuf` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("transport/websocket-protobuf"))
  .settings(
    commons,
    protobuf,
    PB.protoSources in Compile := Seq(file("transport/websocket-protobuf/src/main/protobuf"))
  )
  .jsSettings(
    fork in Test := false
  )
  .enablePlugins(AutomateHeaderPlugin)

lazy val `websocket-protobuf-js` = `websocket-protobuf`.js
lazy val `websocket-protobuf-jvm` = `websocket-protobuf`.jvm

lazy val `transport-grpc-proxy` = project
  .in(file("transport/grpc-proxy"))
  .settings(
    commons,
    grpc,
    PB.protoSources in Compile := Seq(
      file(baseDirectory.value.absolutePath + "/src/test/protobuf/")
    ),
    libraryDependencies ++= Seq(
      http4sDsl,
      http4sBlazeServer,
      slogging,
      monix3,
      fs2ReactiveStreams,
      fluenceCodec,
      scalatest
    )
  )
  .dependsOn(`transport-core-jvm`, `websocket-protobuf-jvm`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `transport-websocket-js` = project
  .in(file("transport/websocket-js"))
  .settings(
    commons,
    PB.targets in Compile := Seq(
      scalapb.gen(flatPackage = true) -> (sourceManaged in Compile).value
    ),
    libraryDependencies ++= Seq(
      "io.monix"      %%% "monix"       % MonixV,
      "biz.enef"      %%% "slogging"    % SloggingV,
      "org.scala-js"  %%% "scalajs-dom" % scalajsDomV,
      "org.scodec"    %%% "scodec-bits" % ScodecBitsV,
      "one.fluence"   %%% "codec-core"  % CodecV,
      "org.scalatest" %%% "scalatest"   % ScalatestV % Test
    ),
    scalaJSUseMainModuleInitializer := true,
    scalaJSModuleKind in Test       := ModuleKind.CommonJSModule,
    fork in Test                    := false,
    jsEnv                           := new org.scalajs.jsenv.jsdomnodejs.JSDOMNodeJSEnv()
  )
  .enablePlugins(ScalaJSPlugin)
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`websocket-protobuf-js`)

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

lazy val `transport-core-js` = `transport-core`.js
lazy val `transport-core-jvm` = `transport-core`.jvm

lazy val `storage-core` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("storage/core"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "one.fluence"   %%% "codec-core"  % CodecV,
      "org.typelevel" %%% "cats-effect" % CatsEffectV,
      "org.scalatest" %%% "scalatest"   % ScalatestV % Test,
      "io.monix"      %%% "monix"       % MonixV % Test
    )
  )
  .jsSettings(
    fork in Test      := false,
    scalaJSModuleKind := ModuleKind.CommonJSModule
  )
  .enablePlugins(AutomateHeaderPlugin)

lazy val `storage-core-jvm` = `storage-core`.jvm
lazy val `storage-core-js` = `storage-core`.js

lazy val `storage-rocksdb` = project
  .in(file("storage/rocksdb"))
  .dependsOn(`storage-core-jvm`)
  .settings(
    commons,
    libraryDependencies ++= Seq(
      rocksDb,
      typeSafeConfig,
      ficus,
      monix3,
      slogging,
      scalatest,
      mockito
    )
  )
  .enablePlugins(AutomateHeaderPlugin)

// core entities for all b-tree modules
lazy val `b-tree-core` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("b-tree/core"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "one.fluence"   %%% "codec-core"  % CodecV,
      "org.scodec"    %%% "scodec-bits" % ScodecBitsV,
      "org.typelevel" %%% "cats-core"   % Cats1V,
      "org.scalatest" %%% "scalatest"   % ScalatestV % Test
    )
  )
  .jsSettings(
    fork in Test := false
  )
  .enablePlugins(AutomateHeaderPlugin)

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
  .dependsOn(`b-tree-core`, `crypto-core`)

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
    fork in Test      := false,
    scalaJSModuleKind := ModuleKind.CommonJSModule
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`b-tree-common`, `b-tree-protocol`, `crypto-hashsign` % Test)

lazy val `b-tree-client-js` = `b-tree-client`.js
lazy val `b-tree-client-jvm` = `b-tree-client`.jvm

lazy val `b-tree-server` = project
  .in(file("b-tree/server"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "one.fluence" %% "codec-kryo" % CodecV
    )
  )
  .dependsOn(
    `storage-rocksdb`,
    `b-tree-common-jvm`,
    `b-tree-protocol-jvm`,
    `b-tree-client-jvm` % "compile->test"
  )

//////////////////////////////////////////////////////////////
//////////////         CRYPTO BEGIN        ///////////////////
//////////////////////////////////////////////////////////////
lazy val `crypto-core` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("crypto/core"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "one.fluence"   %%% "codec-bits" % CodecV,
      "org.scalatest" %%% "scalatest"  % ScalatestV % Test
    )
  )
  .jsSettings(
    fork in Test := false
  )
  .enablePlugins(AutomateHeaderPlugin)

lazy val `crypto-core-js` = `crypto-core`.js
lazy val `crypto-core-jvm` = `crypto-core`.jvm

lazy val `crypto-keystore` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("crypto/keystore"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "one.fluence"   %%% "codec-circe" % CodecV,
      "biz.enef"      %%% "slogging"    % SloggingV,
      "org.scalatest" %%% "scalatest"   % ScalatestV % Test
    )
  )
  .jsSettings(
    fork in Test := false
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`crypto-core`)

lazy val `crypto-keystore-js` = `crypto-keystore`.js
lazy val `crypto-keystore-jvm` = `crypto-keystore`.jvm

lazy val `crypto-hashsign` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("crypto/hashsign"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "biz.enef"      %%% "slogging"  % SloggingV,
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
      "elliptic" -> "6.4.0"
    ),
    scalaJSModuleKind in Test := ModuleKind.CommonJSModule,
    //all JavaScript dependencies will be concatenated to a single file *-jsdeps.js
    skip in packageJSDependencies := false,
    fork in Test                  := false
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`crypto-core`, `crypto-keystore` % Test)

lazy val `crypto-hashsign-js` = `crypto-hashsign`.js
  .enablePlugins(ScalaJSBundlerPlugin)
lazy val `crypto-hashsign-jvm` = `crypto-hashsign`.jvm

lazy val `crypto-cipher` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("crypto/cipher"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
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
      "crypto-js" -> "3.1.9-1"
    ),
    //all JavaScript dependencies will be concatenated to a single file *-jsdeps.js
    skip in packageJSDependencies := false,
    fork in Test                  := false,
    scalaJSModuleKind             := ModuleKind.CommonJSModule
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`crypto-hashsign`)

lazy val `crypto-cipher-js` = `crypto-cipher`.js
  .enablePlugins(ScalaJSBundlerPlugin)
lazy val `crypto-cipher-jvm` = `crypto-cipher`.jvm

//////////////////////////////////////////////////////////////
//////////////          CRYPTO END         ///////////////////
//////////////////////////////////////////////////////////////

lazy val `dataset-node` = project
  .in(file("dataset/node"))
  .dependsOn(
    `storage-core-jvm`,
    `kademlia-core-jvm`,
    `b-tree-server`,
    `dataset-client-jvm`,
    `b-tree-client-jvm`,
    `dataset-protobuf-jvm`
  )

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

lazy val `dataset-protobuf` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("dataset/protobuf"))
  .settings(
    commons,
    protobuf,
    PB.protoSources in Compile := Seq(file("dataset/protobuf/src/main/protobuf"))
  )
  .jsSettings(
    fork in Test := false
  )
  .enablePlugins(AutomateHeaderPlugin)

lazy val `dataset-protobuf-jvm` = `dataset-protobuf`.jvm
lazy val `dataset-protobuf-js` = `dataset-protobuf`.js

lazy val `dataset-grpc` = project
  .in(file("dataset/grpc"))
  .settings(
    commons,
    grpc,
    PB.protoSources in Compile := Seq(file("dataset/grpc/src/main/protobuf"))
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`dataset-client-jvm`, `dataset-node`, `transport-grpc-jvm`)

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
  .dependsOn(`dataset-protocol`, `crypto-core`, `b-tree-client`, `kademlia-core`, `dataset-protobuf`)

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
  .settings(
    commons,
    libraryDependencies ++= Seq(
      scalatest
    )
  )
  .jsSettings(
    fork in Test      := false,
    scalaJSModuleKind := ModuleKind.CommonJSModule
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`contract-protocol`, `crypto-core`)

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

lazy val `contract-protobuf` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("contract/protobuf"))
  .settings(
    commons,
    protobuf,
    PB.protoSources in Compile := Seq(file("contract/protobuf/src/main/protobuf"))
  )
  .jsSettings(
    fork in Test := false
  )
  .enablePlugins(AutomateHeaderPlugin)

lazy val `contract-protobuf-js` = `contract-protobuf`.js
lazy val `contract-protobuf-jvm` = `contract-protobuf`.jvm

lazy val `contract-grpc` = project
  .in(file("contract/grpc"))
  .settings(
    commons,
    grpc,
    PB.protoSources in Compile := Seq(file("contract/grpc/src/main/protobuf")),
    libraryDependencies ++= Seq(
      scalatest
    )
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`contract-core-jvm`, `transport-grpc-jvm`, `contract-protobuf-jvm`, `kademlia-grpc-jvm`)

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
  .dependsOn(`kademlia-monix`, `contract-client`, `dataset-client`, `transport-core`, `crypto-cipher`)

lazy val `client-core-js` = `client-core`.js
lazy val `client-core-jvm` = `client-core`.jvm

lazy val `client-grpc` = project
  .in(file("client/grpc"))
  .settings(commons)
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`client-core-jvm`, `transport-grpc-jvm`, `kademlia-grpc-jvm`, `dataset-grpc`, `contract-grpc`)

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
  .dependsOn(`client-cli-jvm`, `client-grpc`, `crypto-keystore-jvm`)

lazy val `node-core` = project
  .in(file("node/core"))
  .settings(
    commons,
    protobuf,
    PB.protoSources in Compile := {
      Seq(file("node/core/src/main/protobuf"))
    },
    libraryDependencies ++= Seq(
      "one.fluence" %% "codec-protobuf" % CodecV,
      scalatest
    )
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`dataset-node`, `contract-node`, `client-core-jvm`)

lazy val `node-grpc` = project
  .in(file("node/grpc"))
  .settings(
    commons
  )
  .dependsOn(`node-core`, `client-grpc`, `transport-grpc-proxy`)

lazy val `node` = project
  .dependsOn(`node-grpc`, `crypto-keystore-jvm`)
