import SbtCommons._
import VmSbt._
import sbt.Scoped.AnyInitTask

import scala.sys.process._

name := "fluence"

commons

/* Projects */

lazy val `vm` = (project in file("vm"))
  .configs(IntegrationTest)
  .settings(inConfig(IntegrationTest)(Defaults.itSettings): _*)
  .settings(
    commons,
    libraryDependencies ++= Seq(
      cats,
      catsEffect,
      ficus,
      cryptoHashsign,
      scalaTest,
      scalaIntegrationTest,
      mockito
    ),
    compileFrank  := compileFrankTask.value,
    downloadLlama := downloadLlama(resourceDirectory in IntegrationTest).value,
  )
  .settings(itDepends(test)(downloadLlama, compileFrank)(Test, IntegrationTest): _*)
  .settings(itDepends(testOnly)(downloadLlama, compileFrank)(Test, IntegrationTest): _*)
  .dependsOn(`log`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `statemachine` = (project in file("statemachine"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      scalaTest
    )
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(
    `vm`,
    `statemachine-api`
  )

lazy val `statemachine-api` = (project in file("statemachine/api"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      scodecBits,
      circeGeneric,
      fs2,
      cats,
      shapeless
    )
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`log`, `effects`)

lazy val `statemachine-http` = (project in file("statemachine/http"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      http4sDsl,
      http4sCirce
    )
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`statemachine-api`)

lazy val `statemachine-client` = (project in file("statemachine/client"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      sttpCirce,
      scalaTest,
      http4sServer % Test
    )
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(
    `statemachine-api`,
    `sttp-effect`,
    `statemachine-http` % Test,
    `statemachine-abci` % Test,
    `statemachine`      % Test
  )

lazy val `statemachine-abci` = (project in file("statemachine/abci"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      "com.github.jtendermint" % "jabci" % "0.26.0"
    )
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`statemachine-api`)

lazy val `statemachine-docker` = (project in file("statemachine/docker"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      http4sServer,
      scalaTest
    ),
    assemblyJarName in assembly       := "statemachine.jar",
    assemblyMergeStrategy in assembly := SbtCommons.mergeStrategy.value,
    test in assembly                  := {},
    parallelExecution in Test         := false,
    docker                            := { runCmd(s"make worker TAG=v${version.value}") },
    docker in Test                    := { assembly.value; runCmd("make worker-test") },
    assembly                          := assembly.dependsOn(makeFrankSoLib(baseDirectory in `vm`)).value,
    compile in Test                   := (compile in Test).dependsOn(downloadLlama(resourceDirectory in IntegrationTest in `vm`)).value
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`statemachine-http`, `statemachine-abci`, `statemachine`, `sttp-effect` % Test)

lazy val `statemachine-docker-client` = (project in file("statemachine/docker-client"))
  .settings(
    commons
  )
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(`statemachine-client`, `dockerio`)

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

lazy val `sttp-effect` = (project in file("effects/sttp"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      cats,
      catsEffect,
      sttp,
      sttpFs2Backend,
      sttpCatsBackend
    )
  )
  .dependsOn(`effects`)
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
  .dependsOn(`ca-store`, `sttp-effect` % "test->test;compile->compile")
  .enablePlugins(AutomateHeaderPlugin)

lazy val `ipfs` = (project in file("effects/ipfs"))
  .settings(
    commons,
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
  .dependsOn(`ca-store`, `sttp-effect`)
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
    )
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
  .dependsOn(`effects`, `sttp-effect`, `tendermint-block`, `log`, `tendermint-block-history`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `tendermint-block` = (project in file("history/tendermint-block"))
  .settings(
    commons,
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
      levelDb,
      scalaTest
    )
  )
  .dependsOn(`effects`, `tendermint-block`, `ipfs`, `kvstore`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `receipt-storage` = (project in file("history/receipt-storage"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      fs2,
      fs2io,
      cats,
      catsEffect,
      scalaTest
    )
  )
  .dependsOn(`log`, `kvstore`, `tendermint-block-history`, `kademlia`, `kademlia-dht`)

lazy val `kademlia` = project
  .in(file("kademlia"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      cats,
      catsEffect,
      codecCore,
      cryptoHashsign,
      catsTestkit,
      scalaTest,
      disciplineScalaTest,
      scalacheckShapeless
    )
  )
  .dependsOn(`kvstore`, `log`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `kademlia-http` = (project in file("kademlia/http"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      circeGeneric,
      circeParser,
      http4sDsl,
      scalaTest,
      http4sServer % Test
    )
  )
  .dependsOn(`kademlia`, `kademlia-dht`, `sttp-effect`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `kademlia-dht` = (project in file("kademlia/dht"))
  .settings(
    commons
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
    // add classes from Test to dependencyClasspath of IntegrationTest, so it is possible to share Eventually trait
    dependencyClasspath in IntegrationTest := (dependencyClasspath in IntegrationTest).value ++ (exportedProducts in Test).value,
    mainClass in assembly                  := Some("fluence.node.MasterNodeApp"),
    assemblyJarName in assembly            := "master-node.jar",
    test in assembly                       := {},
    docker                                 := { runCmd(s"make worker TAG=v${version.value}") },
    docker in Test                         := { assembly.value; runCmd("make node-test") }
  )
  .settings({
    val tasks = Seq[AnyInitTask](
      docker in Test,
      docker in Test in `statemachine-docker`,
      downloadLlama(resourceDirectory in IntegrationTest in `vm`),
      compile in IntegrationTest
    )
    itDepends(test)(tasks: _*)(IntegrationTest) ++
      itDepends(testOnly)(tasks: _*)(IntegrationTest)
  }: _*)
  .settings(buildContractBeforeDocker())
  .enablePlugins(AutomateHeaderPlugin)
  .dependsOn(
    `ethclient`,
    `swarm`,
    `ipfs`,
    `statemachine-docker-client`,
    `kvstore`,
    `dockerio`,
    `tendermint-rpc`,
    `tendermint-rpc`           % "test->test",
    `tendermint-rpc`           % "it->test",
    `tendermint-block`         % "test->test",
    `tendermint-block-history` % "test->test",
    `sttp-effect`,
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
  .dependsOn(
    `node`           % "test->test",
    `node`           % "test->it",
    `statemachine`   % "test->test",
    `tendermint-rpc` % "test->test"
  )
  .enablePlugins(AutomateHeaderPlugin)
