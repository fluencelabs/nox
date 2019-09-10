import sbt._
import sbtdocker.DockerPlugin.autoImport.Dockerfile

object DockerContainers {
  val FrunRust = "fluencelabs/frun-rust"

  def frunRust(artifact: sbt.File, resourceDirectory: sbt.File): Dockerfile = {
    val port = 30000
    val artifactTargetPath = s"/${artifact.name}"

    // TODO: separate rust + openjdk to a separate container,
    //  so we don't waste time rebuilding these layers every time
    new Dockerfile {
      from("openjdk:8-jre-slim")

      env("PATH", "/root/.cargo/bin/:$PATH")

      val update = "apt-get -qq update >/dev/null"
      val curlInstall = "apt-get -qq install -yq --no-install-recommends curl clang >/dev/null"
      val rustup = "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly"
      val wasm = "/root/.cargo/bin/rustup target add wasm32-unknown-unknown --toolchain nightly"
      val curlRm = "apt-get remove -yq --auto-remove curl >/dev/null"

      runRaw(s"$update && $curlInstall && ($rustup) && $wasm && $curlRm")

      expose(port)

      copy(resourceDirectory / "reference.conf", "/reference.conf")
      copy(artifact, artifactTargetPath)

      entryPoint("java", "-jar", "-Dconfig.file=/reference.conf", "-Xmx2G", artifactTargetPath)
    }
  }

  val Frun = "fluencelabs/frun"

  def frun(artifact: sbt.File, resourceDirectory: sbt.File): Dockerfile = {
    val port = 30000
    val artifactTargetPath = s"/${artifact.name}"

    new Dockerfile {
      from("openjdk:8-jre-alpine")

      expose(port)

      copy(resourceDirectory / "reference.conf", "/reference.conf")
      copy(artifact, artifactTargetPath)

      entryPoint("java", "-jar", "-Dconfig.file=/reference.conf", "-Xmx2G", artifactTargetPath)
    }
  }

  val Worker = "fluencelabs/worker"

  def worker(artifact: sbt.File, baseDirectory: sbt.File): Dockerfile = {
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
      copy(baseDirectory / "worker", workerDataRoot)
      copy(artifact, artifactTargetPath)

      entryPoint("sh", workerRunScript, artifactTargetPath)
    }
  }

  val Node = "fluencelabs/node"

  def node(artifact: sbt.File, resourceDirectory: sbt.File): Dockerfile = {
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
      copy(resourceDirectory / "docker", "/")
      copy(artifact, artifactTargetPath)

      cmd("java", "-jar", "-Dconfig.file=/master/application.conf", artifactTargetPath)
      entryPoint("sh", "/entrypoint.sh")
    }
  }
}
