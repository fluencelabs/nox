import SbtCommons._
import sbt.Keys._
import sbt._

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

lazy val vm = (project in file("vm"))
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
      mockito,
      slogging
    ),
    test in IntegrationTest := (test in IntegrationTest)
      .dependsOn(compile in `vm-counter`)
      .dependsOn(compile in `vm-hello-world`)
      .dependsOn(compile in `vm-llamadb`)
      .value
  )
  .dependsOn(`merkelized-bytebuffer`)
  .enablePlugins(AutomateHeaderPlugin)

/**
 * Wasm VM docker runner for easy Wasm app debugging
 */
lazy val frun = (project in file("vm/frun"))
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
    .dependsOn(vm, statemachine)
    .enablePlugins(AutomateHeaderPlugin, DockerPlugin)

lazy val `frun-rust` = project.in(frun.base / "rust").settings(
  imageNames in docker := Seq(ImageName(DockerContainers.FrunRust)),
  dockerfile in docker := DockerContainers.frunRust((assembly in frun).value, (resourceDirectory in frun in Compile).value)
).dependsOn(frun).enablePlugins(DockerPlugin)

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
  .dependsOn(vm, `vm-hello-world`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `merkelized-bytebuffer` = (project in file("statemachine/merkelized-bytebuffer"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      asmble,
      cryptoHashing,
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
      slogging,
      http4sDsl,
      http4sServer,
      http4sCirce,
      scalaTest,
      sttp % Test,
      sttpCirce % Test,
      sttpCatsBackend % Test
    )
  )

lazy val statemachine = (project in file("statemachine"))
  .settings(
    commons,
    kindProjector,
    libraryDependencies ++= Seq(
      pureConfig,
      slogging,
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
  .dependsOn(vm, `statemachine-control`, `tendermint-rpc`, sttpEitherT, `tendermint-block`)

lazy val effects = project
  .settings(
    commons,
    libraryDependencies ++= Seq(
      cats,
      catsEffect
    )
  )
  .enablePlugins(AutomateHeaderPlugin)

lazy val sttpEitherT = (project in file("effects/sttpEitherT"))
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
  .dependsOn(effects)
  .enablePlugins(AutomateHeaderPlugin)

lazy val swarm = (project in file("effects/swarm"))
  .settings(
    commons,
    kindProjector,
    libraryDependencies ++= Seq(
      sttp,
      sttpCirce,
      sttpCatsBackend % Test,
      sttpFs2Backend % Test,
      slogging,
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
  .dependsOn(`ca-store`, sttpEitherT % "test->test;compile->compile")
  .enablePlugins(AutomateHeaderPlugin)

lazy val ipfs = (project in file("effects/ipfs"))
  .settings(
    commons,
    kindProjector,
    libraryDependencies ++= Seq(
      sttp,
      sttpCirce,
      circeGeneric,
      circeFs2,
      slogging,
      scodecBits,
      scodecCore,
      scalaTest
    )
  )
  .dependsOn(`ca-store`, sttpEitherT % "test->test;compile->compile")
  .enablePlugins(AutomateHeaderPlugin)

lazy val ethclient = (project in file("effects/ethclient"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      web3jCore,
      slogging,
      scodecBits,
      fs2,
      fs2rx,
      scalaTest
    ),
  )
  .dependsOn(effects)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `kvstore` = (project in file("effects/kvstore"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      slogging,
      codecCore,
      fs2,
      rocksDb,
      scalaTest
    )
  )
  .dependsOn(effects)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `dockerio` = (project in file("effects/docker"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      slogging
    )
  )
  .dependsOn(effects)
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
      slogging
    )
  )
  .dependsOn(effects)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `tendermint-block` = (project in file("effects/tendermint-block"))
  .settings(
    commons,
    kindProjector,
    libraryDependencies ++= Seq(
      circeGeneric,
      circeParser,
      circeGenericExtras,
      slogging,
      protobuf,
      protobufUtil,
      scodecBits,
      cryptoHashsign,
      scalaTest,
      bouncyCastle
    )
  )
  .dependsOn(effects)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `kademlia` = (project in file("kademlia"))
  .settings(
    commons,
    kindProjector,
    libraryDependencies ++= Seq(
      slogging,
      cats,
      catsEffect,
      codecCore,
      cryptoJwt,
      cryptoHashsign,
      scalaTest
    )
  )
  .dependsOn(`kvstore`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val node = project
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
    test in IntegrationTest := (test in IntegrationTest)
      .dependsOn(docker)
      .dependsOn(docker in statemachine)
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
  .dependsOn(ethclient, swarm, ipfs, `statemachine-control`, `kvstore`, `dockerio`, `tendermint-rpc`, sttpEitherT)
