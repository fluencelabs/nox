import SbtCommons._

name := "fluence"

commons

// Limit all tasks parallelism to 1, to lower memory consumption on CI
// Reason: CircleCI provides 4Gb instances, and with concurrent compilations (Scala + Rust) and tests it runs out of memory limit
// TODO: limits should be more granular. E.g., there's no reason to limit network tasks
Global / concurrentRestrictions := Seq(
  Tags.limitAll(1)
).filter(_ => sys.env.get("CI").contains("true"))

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
    kindProjector,
    libraryDependencies ++= Seq(
      asmble,
      cats,
      catsEffect,
      ficus,
      cryptoHashsign,
      scalaTest,
      scalaIntegrationTest,
      mockito
    ),
    test in IntegrationTest := (test in IntegrationTest)
      .dependsOn(compile in `vm-counter`)
      .dependsOn(compile in `vm-hello-world`)
      .dependsOn(compile in `vm-llamadb`)
      .dependsOn(compile in `vm-instrumented-llamadb`)
      .value
  )
  .dependsOn(`merkelized-bytebuffer`, `log`)
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
    imageNames in docker              := Seq(ImageName(DockerContainers.Frun)),
    dockerfile in docker              := DockerContainers.frun(assembly.value, (resourceDirectory in Compile).value)
  )
  .dependsOn(`vm`, `statemachine`)
  .enablePlugins(AutomateHeaderPlugin, DockerPlugin)

lazy val `frun-rust` = project
  .in(frun.base / "rust")
  .settings(
    imageNames in docker := Seq(ImageName(DockerContainers.FrunRust)),
    dockerfile in docker := DockerContainers.frunRust((assembly in `frun`).value,
                                                      (resourceDirectory in `frun` in Compile).value)
  )
  .dependsOn(`frun`)
  .enablePlugins(DockerPlugin)

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

lazy val `vm-instrumented-llamadb` = (project in file("vm/src/it/resources/test-cases/instrumented-llamadb"))
  .settings(
    createInstrumentedLlamadb()
  )

lazy val `vm-hello-world-runner` = (project in file("vm/src/it/resources/test-cases/hello-world/runner"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      asmble,
      cats,
      catsEffect
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
    kindProjector,
    libraryDependencies ++= Seq(
      cats,
      catsEffect,
      circeGeneric,
      circeParser,
      http4sDsl,
      http4sServer,
      http4sCirce,
      scalaTest,
      sttp            % Test,
      sttpCirce       % Test,
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
      scodecBits,
      "com.github.jtendermint" % "jabci" % "0.26.0",
      scalaTest
    ),
    assemblyJarName in assembly       := "statemachine.jar",
    assemblyMergeStrategy in assembly := SbtCommons.mergeStrategy.value,
    test in assembly                  := {},
    imageNames in docker              := Seq(ImageName(DockerContainers.Worker)),
    dockerfile in docker              := DockerContainers.worker(assembly.value, baseDirectory.value)
  )
  .enablePlugins(AutomateHeaderPlugin, DockerPlugin)
  .dependsOn(
    `vm`,
    `statemachine-control`,
    `statemachine-data`,
    `tendermint-rpc`,
    `sttpEitherT`,
    `tendermint-block`
  )

lazy val `statemachine-data` = (project in file("statemachine/data"))
  .settings(
    commons,
    kindProjector,
    libraryDependencies ++= Seq(
      scodecBits,
      cats
    )
  )
  .enablePlugins(AutomateHeaderPlugin, DockerPlugin)
  .dependsOn(`log`)

lazy val `effects` = project
  .in(file("effects"))
  .settings(
    commons,
    fork in Test := false,
    libraryDependencies ++= Seq(
      cats,
      catsEffect
    )
  )
  .dependsOn(`log`)
  .enablePlugins(AutomateHeaderPlugin)

// TODO this is not an `effect`
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
  .enablePlugins(AutomateHeaderPlugin)

lazy val `ca-store` = (project in file("effects/ca-store"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      scodecCore,
      fs2,
      fs2io
    )
  )
  .dependsOn(`effects`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `swarm` = (project in file("effects/swarm"))
  .settings(
    commons,
    kindProjector,
    libraryDependencies ++= Seq(
      sttp,
      sttpCirce,
      sttpCatsBackend % Test,
      sttpFs2Backend  % Test,
      circeCore,
      circeGeneric,
      circeGenericExtras,
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
      cryptoHashsign,
      scalaTest
    )
  )
  .dependsOn(`ca-store`, `sttpEitherT` % "test->test;compile->compile")
  .enablePlugins(AutomateHeaderPlugin)

lazy val `ethclient` = (project in file("effects/ethclient"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      web3jCore,
      scodecBits,
      fs2,
      fs2rx,
      scalaTest
    ),
  )
  .dependsOn(`effects`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `kvstore` = project
  .in(file("effects/kvstore"))
  .settings(
    commons,
    fork in Test := false,
    libraryDependencies ++= Seq(
      codecCore,
      fs2,
      scalaTest,
      rocksDb
    )
  )
  .dependsOn(`effects`, `log`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `dockerio` = (project in file("effects/docker"))
  .settings(
    commons
  )
  .dependsOn(`effects`)
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
      http4sDsl       % Test,
      http4sServer    % Test,
      sttp            % Test,
      sttpCirce       % Test,
      sttpCatsBackend % Test
    )
  )
  .dependsOn(`effects`, `sttpEitherT`, `tendermint-block`, `log`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `tendermint-block` = (project in file("history/tendermint-block"))
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
  .dependsOn(`effects`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `tendermint-block-history` = (project in file("history/tendermint-block-history"))
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
  .dependsOn(`effects`, `tendermint-block`, `ipfs`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `receipt-storage` = (project in file("history/receipt-storage"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      fs2,
      fs2io,
      cats,
      catsEffect,
      scalaTest,
    )
  )
  .dependsOn(`log`, `kvstore`, `tendermint-block-history`)

lazy val `kademlia` = project
  .in(file("kademlia"))
  .settings(
    commons,
    kindProjector,
    libraryDependencies ++= Seq(
      cats,
      catsEffect,
      codecCore,
      cryptoHashsign,
      catsTestkit,
      scalaTest,
      "com.github.alexarchambault" %% "scalacheck-shapeless_1.13" % "1.1.8" % Test
    )
  )
  .dependsOn(`kvstore`, `log`)
  .enablePlugins(AutomateHeaderPlugin)

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
  )
  .dependsOn(`kademlia`, `kademlia-dht`, `kademlia-testkit` % Test)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `kademlia-dht` = (project in file("kademlia/dht"))
  .settings(
    commons,
    kindProjector
  )
  .dependsOn(`kademlia`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `kademlia-testkit` = (project in file("kademlia/testkit"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      scalaTest
    )
  )
  .dependsOn(`kademlia`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `log` = project
  .in(file("log"))
  .settings(
    commons,
    fork in Test := false,
    kindProjector,
    libraryDependencies ++= Seq(
      cats,
      catsEffect,
      scalaTest
    )
  )
  .enablePlugins(AutomateHeaderPlugin)

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
    // add classes from Test to dependencyClasspath of IntegrationTest, so it is possible to share Eventually trait
    dependencyClasspath in IntegrationTest := (dependencyClasspath in IntegrationTest).value ++ (exportedProducts in Test).value,
    mainClass in assembly                  := Some("fluence.node.MasterNodeApp"),
    assemblyJarName in assembly            := "master-node.jar",
    test in assembly                       := {},
    imageNames in docker                   := Seq(ImageName(DockerContainers.Node)),
    dockerfile in docker                   := DockerContainers.node(assembly.value, (resourceDirectory in Compile).value)
  )
  .settings(buildContractBeforeDocker())
  .enablePlugins(AutomateHeaderPlugin, DockerPlugin)
  .dependsOn(
    `ethclient`,
    `swarm`,
    `ipfs`,
    `statemachine-control`,
    `statemachine-data`,
    `kvstore`,
    `dockerio`,
    `tendermint-rpc`,
    `tendermint-rpc`           % "test->test",
    `tendermint-rpc`           % "it->test",
    `tendermint-block`         % "test->test",
    `tendermint-block-history` % "test->test",
    `sttpEitherT`,
    `receipt-storage`,
    `log`,
    `kademlia-http`,
    `kademlia-testkit` % Test
  )

lazy val `node-testkit` = (project in file("node/testkit"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      scalaTest
    )
  )
  .dependsOn(`node` % "test->test")
  .dependsOn(`statemachine` % "test->test")
  .dependsOn(`tendermint-rpc` % "test->test")
  .enablePlugins(AutomateHeaderPlugin)
