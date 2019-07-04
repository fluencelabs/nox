import SbtCommons._
import org.scalajs.sbtplugin.ScalaJSPlugin.autoImport._
import sbtcrossproject.CrossPlugin.autoImport.crossProject

name := "fluence"

commons

onLoad in Global := (onLoad in Global).value.andThen { state ⇒
  val requiredVersion = "1.8" // Asmble works only on Java 8.
  val currentVersion = sys.props("java.specification.version")
  assert(currentVersion == requiredVersion,
    s"Unsupported $currentVersion JDK version, please use $requiredVersion JDK version instead.")

  state
}

/* Projects */

lazy val `vm` = (project in file("vm"))
  .configs(IntegrationTest)
  .settings(inConfig(IntegrationTest)(Defaults.itSettings): _*)
  .settings(
    commons,
    libraryDependencies ++= Seq(
      asmble,
      cats,
      catsEffect,
      pureConfig,
      cryptoHashsign,
      scalaTest,
      scalaIntegrationTest,
      mockito
    ),
    test in IntegrationTest := (test in IntegrationTest)
      .dependsOn(compile in `vm-counter`)
      .dependsOn(compile in `vm-hello-world`)
      .dependsOn(compile in `vm-llamadb`)
      .value
  )
  .dependsOn(`merkelized-bytebuffer`, `log-jvm`)
  .enablePlugins(AutomateHeaderPlugin)

/**
 * Wasm VM docker runner for easy Wasm app debugging
 */
lazy val `frun` = (project in file("vm/frun"))
    .settings(
      commons,
      libraryDependencies ++= Seq(
        asmble,
        cats,
        catsEffect,
        sttp,
        sttpCirce,
        sttpCatsBackend,
        http4sDsl,
        http4sServer,
      ),
      assemblyMergeStrategy in assembly := SbtCommons.mergeStrategy.value,
      imageNames in docker := Seq(ImageName(DockerContainers.Frun)),
      dockerfile in docker := DockerContainers.frun(assembly.value, (resourceDirectory in Compile).value)
    )
    .dependsOn(`vm`, `statemachine`)
    .enablePlugins(AutomateHeaderPlugin, DockerPlugin)

lazy val `frun-rust` = project.in(frun.base / "rust").settings(
  imageNames in docker := Seq(ImageName(DockerContainers.FrunRust)),
  dockerfile in docker := DockerContainers.frunRust((assembly in `frun`).value, (resourceDirectory in `frun` in Compile).value)
).dependsOn(`frun`).enablePlugins(DockerPlugin)

lazy val `vm-counter` = (project in file("vm/src/it/resources/test-cases/counter"))
  .settings(
    rustVmTest("counter")
  )

lazy val `vm-hello-world` = (project in file("vm/src/it/resources/test-cases/hello-world"))
  .settings(
    rustVmTest("hello-world")
  )

lazy val `vm-llamadb` = (project in file("vm/src/it/resources/test-cases/llamadb"))
  .settings(
    rustVmTest("llamadb")
  )

lazy val `vm-hello-world-runner` = (project in file("vm/src/it/resources/test-cases/hello-world/runner"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      asmble,
      cats,
      catsEffect,
      pureConfig
    )
  )
  .dependsOn(`vm`, `vm-hello-world`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `merkelized-bytebuffer` = (project in file("vm/merkelized-bytebuffer"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      asmble,
      cryptoHashsign,
      scalaTest
    )
  )

lazy val `statemachine-control` = (project in file("statemachine/control"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      cats,
      catsEffect,
      circeGeneric,
      circeParser,
      http4sDsl,
      http4sServer,
      http4sCirce,
      scalaTest,
      sttp % Test,
      sttpCirce % Test,
      sttpCatsBackend % Test
    )
  )
  .dependsOn(`tendermint-block-history`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `statemachine` = (project in file("statemachine"))
  .settings(
    commons,
    kindProjector,
    libraryDependencies ++= Seq(
      pureConfig,
      scodecBits,
      "com.github.jtendermint" % "jabci" % "0.26.0",
      scalaTest
    ),
    assemblyJarName in assembly := "statemachine.jar",
    assemblyMergeStrategy in assembly := SbtCommons.mergeStrategy.value,
    test in assembly     := {},
    imageNames in docker := Seq(ImageName(DockerContainers.Worker)),
    dockerfile in docker := DockerContainers.worker(assembly.value, baseDirectory.value)
  )
  .enablePlugins(AutomateHeaderPlugin, DockerPlugin)
  .dependsOn(`vm`, `statemachine-control`, `tendermint-rpc`, `sttpEitherT`, `tendermint-block`)

lazy val `effects` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("effects"))
  .settings(
    commons,
    fork in Test := false,
    libraryDependencies ++= Seq(
      "org.typelevel"       %%% "cats-core"   % catsVersion,
      "org.typelevel" %%% "cats-effect" % catsEffectVersion
    )
  )
  .dependsOn(`log`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `effects-js` = `effects`.js
lazy val `effects-jvm` = `effects`.jvm

lazy val `sttpEitherT` = (project in file("effects/sttpEitherT"))
  .settings(
    commons,
    kindProjector,
    libraryDependencies ++= Seq(
      cats,
      catsEffect,
      sttp,
      sttpFs2Backend,
      sttpCatsBackend
    )
  )

lazy val `ca-store` = (project in file("effects/ca-store"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      scodecCore,
      fs2,
      fs2io
    )
  )
  .dependsOn(`effects-jvm`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `swarm` = (project in file("effects/swarm"))
  .settings(
    commons,
    kindProjector,
    libraryDependencies ++= Seq(
      sttp,
      sttpCirce,
      sttpCatsBackend % Test,
      sttpFs2Backend % Test,
      circeCore,
      circeGeneric,
      circeGenericExtras,
      pureConfig,
      scodecBits,
      scodecCore,
      web3jCrypto,
      cryptoHashsign,
      scalaTest
    )
  )
  .dependsOn(`ca-store`, `sttpEitherT` % "test->test;compile->compile")
  .enablePlugins(AutomateHeaderPlugin)

lazy val `ipfs` = (project in file("effects/ipfs"))
  .settings(
    commons,
    kindProjector,
    libraryDependencies ++= Seq(
      sttp,
      sttpCirce,
      circeGeneric,
      circeFs2,
      scodecBits,
      scodecCore,
      scalaTest
    )
  )
  .dependsOn(`ca-store`, `sttpEitherT` % "test->test;compile->compile")
  .enablePlugins(AutomateHeaderPlugin)

lazy val `ethclient` = (project in file("effects/ethclient"))
  .settings(
    commons,
    fork in Test := true,
    libraryDependencies ++= Seq(
      web3jCore,
      scodecBits,
      fs2,
      fs2rx,
      scalaTest
    ),
  )
  .dependsOn(`effects-jvm`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `kvstore` =
  crossProject(JVMPlatform, JSPlatform)
    .withoutSuffixFor(JVMPlatform)
    .crossType(FluenceCrossType)
    .in(file("effects/kvstore"))
    .settings(
      commons,
      fork in Test := false,
      libraryDependencies ++= Seq(
        "one.fluence" %%% "codec-core" % codecVersion,
        "co.fs2" %%% "fs2-core" % fs2Version,
        "org.scalatest" %%% "scalatest" % "3.0.5" % Test
      )
    )
    .jvmSettings(
      libraryDependencies ++= Seq(
        rocksDb
      )
    )
    .dependsOn(`effects`, `log`)
    .enablePlugins(AutomateHeaderPlugin)

lazy val `kvstore-js` = `kvstore`.js
lazy val `kvstore-jvm` = `kvstore`.jvm

lazy val `dockerio` = (project in file("effects/docker"))
  .settings(
    commons
  )
  .dependsOn(`effects-jvm`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `tendermint-rpc` = (project in file("effects/tendermint-rpc"))
  .settings(
    commons,
    kindProjector,
    libraryDependencies ++= Seq(
      sttp,
      circeGeneric,
      circeParser,
      circeGenericExtras,
      fs2,
      fs2io,
      asyncHttpClient,
      scalaTest,
      http4sDsl % Test,
      http4sServer % Test,
      sttp % Test,
      sttpCirce % Test,
      sttpCatsBackend % Test
    )
  )
  .dependsOn(`effects-jvm`, `sttpEitherT`, `tendermint-block` % "test")
  .enablePlugins(AutomateHeaderPlugin)

// TODO remove from effects to history
lazy val `tendermint-block` = (project in file("effects/tendermint-block"))
  .settings(
    commons,
    kindProjector,
    libraryDependencies ++= Seq(
      circeGeneric,
      circeParser,
      circeGenericExtras,
      protobuf,
      protobufUtil,
      scodecBits,
      cryptoHashsign,
      scalaTest,
      bouncyCastle
    )
  )
  .dependsOn(`effects-jvm`)
  .enablePlugins(AutomateHeaderPlugin)

// TODO remove from effects to history
lazy val `tendermint-block-history` = (project in file("effects/tendermint-block-history"))
  .settings(
    commons,
    kindProjector,
    libraryDependencies ++= Seq(
      cats,
      catsEffect,
      sttp,
      circeGeneric,
      circeParser,
      circeGenericExtras,
      scodecBits,
      http4sDsl,
      http4sServer,
      http4sCirce,
      scalaTest,
    )
  )
  .dependsOn(`effects-jvm`, `tendermint-block`, `ipfs`)
  .enablePlugins(AutomateHeaderPlugin)

// TODO remove from effects to history
lazy val `receipt-storage` = (project in file("effects/receipt-storage"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      fs2,
      fs2io,
      cats,
      catsEffect,
      scalaTest,
    )
  ).dependsOn(`log-jvm`, `kvstore-jvm`, `tendermint-block-history`)

lazy val `kademlia` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("kademlia"))
  .settings(
    commons,
    kindProjector,
    libraryDependencies ++= Seq(
      "org.typelevel" %%% "cats-core" % catsVersion,
      "org.typelevel" %%% "cats-effect" % catsEffectVersion,
      "one.fluence" %%% "codec-core" % codecVersion,
      "one.fluence" %%% "crypto-hashsign" % cryptoVersion,
      "org.typelevel" %%% "cats-testkit" % catsVersion % Test,
      "org.scalatest" %%% "scalatest" % "3.0.5"  % Test,
      "com.github.alexarchambault" %%% "scalacheck-shapeless_1.13" % "1.1.8" % Test
    ),
    javacOptions += "-Xmx1G"
  )
  .jsSettings(
    test in Test := {}
  )
  .dependsOn(`kvstore`, `log`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `kademlia-js` = `kademlia`.js.enablePlugins(ScalaJSBundlerPlugin)
lazy val `kademlia-jvm` = `kademlia`.jvm

lazy val `kademlia-http` = (project in file("kademlia/http"))
  .settings(
    commons,
    kindProjector,
    libraryDependencies ++= Seq(
      sttp,
      circeGeneric,
      circeParser,
      http4sDsl,
      scalaTest
    )
  ).dependsOn(`kademlia-jvm`, `kademlia-testkit` % Test)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `kademlia-testkit` = (project in file("kademlia/testkit"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      scalaTest
    )
  ).dependsOn(`kademlia-jvm`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `log` = crossProject(JVMPlatform, JSPlatform)
  .withoutSuffixFor(JVMPlatform)
  .crossType(FluenceCrossType)
  .in(file("log"))
  .settings(
    commons,
    fork in Test := false,
    kindProjector,
    libraryDependencies ++= Seq(
      "org.typelevel" %%% "cats-core" % catsVersion,
      "org.typelevel" %%% "cats-effect" % catsEffectVersion,
      "org.scalatest" %%% "scalatest" % "3.0.5"  % Test
    )
  )
  .enablePlugins(AutomateHeaderPlugin)

lazy val `log-js` = `log`.js
lazy val `log-jvm` = `log`.jvm

lazy val `node` = project
  .configs(IntegrationTest)
  .settings(inConfig(IntegrationTest)(Defaults.itSettings): _*)
  .settings(
    commons,
    kindProjector,
    libraryDependencies ++= Seq(
      fs2io,
      ficus,
      circeGeneric,
      circeParser,
      http4sDsl,
      http4sServer,
      toml,
      scalaIntegrationTest,
      scalaTest
    ),
    assemblyMergeStrategy in assembly := SbtCommons.mergeStrategy.value,
    testOnly in IntegrationTest := (testOnly in IntegrationTest)
      .dependsOn(docker)
      .dependsOn(docker in `statemachine`)
      .dependsOn(compile in `vm-llamadb`)
      .dependsOn(compile in IntegrationTest) // run compilation before building docker containers
      .evaluated,
    test in IntegrationTest := (test in IntegrationTest)
      .dependsOn(docker)
      .dependsOn(docker in `statemachine`)
      .dependsOn(compile in `vm-llamadb`)
      .dependsOn(compile in IntegrationTest) // run compilation before building docker containers
      .value,
    mainClass in assembly       := Some("fluence.node.MasterNodeApp"),
    assemblyJarName in assembly := "master-node.jar",
    test in assembly            := {},
    imageNames in docker        := Seq(ImageName(DockerContainers.Node)),
    dockerfile in docker := DockerContainers.node(assembly.value, (resourceDirectory in Compile).value)
  )
  .settings(buildContractBeforeDocker())
  .enablePlugins(AutomateHeaderPlugin, DockerPlugin)
  .dependsOn(
    `ethclient`, 
    `swarm`, 
    `ipfs`, 
    `statemachine-control`, 
    `kvstore-jvm`,
    `dockerio`, 
    `tendermint-rpc`, 
    `sttpEitherT`, 
    `kademlia-http`,
    `kademlia-testkit` % Test
  )
