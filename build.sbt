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

lazy val `vm-counter` = (project in file("vm/examples/counter"))
  .settings(
    rustVmExample("counter")
  )

lazy val `vm-hello-world` = (project in file("vm/examples/hello-world"))
  .settings(
    rustVmExample("hello-world")
  )

lazy val `vm-hello-world2-2015` = (project in file("vm/examples/hello-world2/app-2015"))
  .settings(
    rustVmExample("hello-world2/app-2015")
  )

lazy val `vm-hello-world2-2018` = (project in file("vm/examples/hello-world2/app-2018"))
  .settings(
    rustVmExample("hello-world2/app-2018")
  )

lazy val `vm-hello-world2-runner` = (project in file("vm/examples/hello-world2/runner"))
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
  .dependsOn(vm, `vm-hello-world2-2015`)
  .dependsOn(vm, `vm-hello-world2-2018`)
  .enablePlugins(AutomateHeaderPlugin)

lazy val `vm-llamadb` = (project in file("vm/examples/llamadb"))
  .settings(
    rustVmExample("llamadb")
  )

lazy val `tic-tac-toe` = (project in file("vm/examples/tic-tac-toe/app"))
  .settings(
    rustVmExample("tic-tac-toe/app")
  )

lazy val `tic-tac-toe-runner` = (project in file("vm/examples/tic-tac-toe/runner"))
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
  .dependsOn(vm, `tic-tac-toe`)
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
      scodecBits,
      http4sDsl,
      http4sServer,
      http4sCirce,
      scalaTest,
      sttp,
      sttpCirce,
      sttpCatsBackend
    )
  )

lazy val statemachine = (project in file("statemachine"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      cats,
      catsEffect,
      circeGeneric,
      circeParser,
      pureConfig,
      cryptoHashing,
      slogging,
      scodecBits,
      prometheusClient,
      prometheusClientJetty,
      prometheusClientServlet,
      "com.github.jtendermint" % "jabci"          % "0.26.0",
      "org.bouncycastle"       % "bcpkix-jdk15on" % "1.56",
      "net.i2p.crypto"         % "eddsa"          % "0.3.0",
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
      val stateMachinePrometheusPort = 26661
      val abciHandlerPort = 26658

      val vmDataRoot = "/vmcode"

      new Dockerfile {
        from("openjdk:8-jre-alpine")

        expose(abciHandlerPort)
        expose(stateMachinePrometheusPort)

        volume(vmDataRoot)

        // includes worker run script
        copy(baseDirectory.value / "docker" / "worker", workerDataRoot)

        copy(artifact, artifactTargetPath)

        entryPoint("sh", workerRunScript, artifactTargetPath)
      }
    }
  )
  .enablePlugins(AutomateHeaderPlugin, DockerPlugin)
  .dependsOn(vm, `statemachine-control`)

lazy val effects = project
  .settings(
    commons,
    libraryDependencies ++= Seq(
      cats,
      catsEffect
    )
  )
  .enablePlugins(AutomateHeaderPlugin)

lazy val swarm = (project in file("effects/swarm"))
  .settings(
    commons,
    libraryDependencies ++= Seq(
      sttp,
      sttpCirce,
      sttpCatsBackend,
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
  .dependsOn(effects)
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

lazy val node = project
  .configs(IntegrationTest)
  .settings(inConfig(IntegrationTest)(Defaults.itSettings): _*)
  .settings(
    commons,
    kindProjector,
    libraryDependencies ++= Seq(
      catsEffect,
      sttp,
      sttpCatsBackend,
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
  .dependsOn(ethclient, swarm, `statemachine-control`, `kvstore`)
