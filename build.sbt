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
      cryptoHashing,
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
  .enablePlugins(AutomateHeaderPlugin)

/**
 * Wasm VM docker runner for easy Wasm app debugging
 */
lazy val flrun = (project in file("vm/flrun"))
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
      assemblyMergeStrategy in assembly := {
        // a module definition fails compilation for java 8, just skip it
        case PathList("module-info.class", xs @ _*) => MergeStrategy.first
        case "META-INF/io.netty.versions.properties" =>
          MergeStrategy.first
        case x =>
          val oldStrategy = (assemblyMergeStrategy in assembly).value
          oldStrategy(x)
      },
      imageNames in docker := Seq(ImageName("fluencelabs/frun")),
      dockerfile in docker := {
        // Run `sbt docker` to create image

        val artifact = assembly.value
        val artifactTargetPath = s"/${artifact.name}"

        val port = 30000

        new Dockerfile {
          from("openjdk:8-jre-alpine")

          expose(port)

          copy((resourceDirectory in Compile).value / "reference.conf", "/reference.conf")
          copy(artifact, artifactTargetPath)

          entryPoint("java", "-jar", "-Dconfig.file=/reference.conf", "-Xmx2G", artifactTargetPath)
        }
      }
    )
    .dependsOn(vm, statemachine)
    .enablePlugins(AutomateHeaderPlugin, DockerPlugin)

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
      pureConfig,
      cryptoHashing,
    )
  )
  .dependsOn(vm, `vm-hello-world`)
  .enablePlugins(AutomateHeaderPlugin)

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
    assemblyMergeStrategy in assembly := {
      // a module definition fails compilation for java 8, just skip it
      case PathList("module-info.class", xs @ _*) => MergeStrategy.first
      case "META-INF/io.netty.versions.properties" =>
        MergeStrategy.first
      case x =>
        val oldStrategy = (assemblyMergeStrategy in assembly).value
        oldStrategy(x)
    },
    test in assembly     := {},
    imageNames in docker := Seq(ImageName("fluencelabs/worker")),
    dockerfile in docker := {
      // Run `sbt docker` to create image

      // The assembly task generates a fat JAR file
      val artifact = assembly.value
      val artifactTargetPath = s"/${artifact.name}"

      // State machine constants
      val workerDataRoot = "/worker"
      val workerRunScript = s"$workerDataRoot/run.sh"
      val abciHandlerPort = 26658

      val vmDataRoot = "/vmcode"

      new Dockerfile {
        from("openjdk:8-jre-alpine")

        expose(abciHandlerPort)

        volume(vmDataRoot)

        // includes worker run script
        copy(baseDirectory.value / "docker" / "worker", workerDataRoot)

        copy(artifact, artifactTargetPath)

        entryPoint("sh", workerRunScript, artifactTargetPath)
      }
    }
  )
  .enablePlugins(AutomateHeaderPlugin, DockerPlugin)
  .dependsOn(vm, `statemachine-control`, `tendermint-rpc`, sttpEitherT)

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
      cryptoHashing,
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
      cryptoHashing,
      scalaTest,
      bouncyCastle
    )
  )
  .dependsOn(effects)
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
    assemblyMergeStrategy in assembly := {
      // a module definition fails compilation for java 8, just skip it
      case PathList("module-info.class", xs @ _*) => MergeStrategy.first
      case "META-INF/io.netty.versions.properties" =>
        MergeStrategy.first
      case x =>
        val oldStrategy = (assemblyMergeStrategy in assembly).value
        oldStrategy(x)
    },
    test in IntegrationTest := (test in IntegrationTest)
      .dependsOn(docker)
      .dependsOn(docker in statemachine)
      .dependsOn(compile in `vm-llamadb`)
      .dependsOn(compile in IntegrationTest) // run compilation before building docker containers
      .value,
    mainClass in assembly       := Some("fluence.node.MasterNodeApp"),
    assemblyJarName in assembly := "master-node.jar",
    test in assembly            := {},
    imageNames in docker        := Seq(ImageName("fluencelabs/node")),
    dockerfile in docker := {
      // The assembly task generates a fat JAR file
      val artifact: File = assembly.value
      val artifactTargetPath = s"/${artifact.name}"

      new Dockerfile {
        // docker is needed in image so it can connect to host's docker.sock and run workers on host
        val dockerBinary = "https://download.docker.com/linux/static/stable/x86_64/docker-18.06.1-ce.tgz"
        from("openjdk:8-jre-alpine")
        runRaw(s"wget -q $dockerBinary -O- | tar -C /usr/bin/ -zxv docker/docker --strip-components=1")

        // this is needed for some binaries (e.g. rocksdb) to run properly on alpine linux since they need libc and
        // alpine use musl
        runRaw("ln -sf /lib/libc.musl-x86_64.so.1 /usr/lib/ld-linux-x86-64.so.2")

        volume("/master") // anonymous volume to store all data

        // p2p ports range
        env("MIN_PORT", "10000")
        env("MAX_PORT", "11000")

        /*
         * The following directory structure is assumed in node/src/main/resources:
         *    docker/
         *      entrypoint.sh
         *      application.conf
         */
        copy((resourceDirectory in Compile).value / "docker", "/")

        copy(artifact, artifactTargetPath)

        cmd("java", "-jar", "-Dconfig.file=/master/application.conf", artifactTargetPath)
        entryPoint("sh", "/entrypoint.sh")
      }
    }
  )
  .settings(buildContractBeforeDocker())
  .enablePlugins(AutomateHeaderPlugin, DockerPlugin)
  .dependsOn(ethclient, swarm, ipfs, `statemachine-control`, `kvstore`, `dockerio`, `tendermint-rpc`, sttpEitherT)
