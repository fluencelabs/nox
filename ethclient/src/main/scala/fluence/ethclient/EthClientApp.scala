package fluence.ethclient

import cats.effect.{ExitCode, IO, IOApp}

object EthClientApp extends IOApp {

  override def run(args: List[String]): IO[ExitCode] =
    EthClient
      .makeHttpResource[IO]()
      .use { ethClient ⇒

        for {
          _ ← IO(println("Launching w3j"))
          version ← ethClient.getClientVersion[IO]()
          _ ← IO(println(s"Client version: $version"))
        } yield ()

      }
      .map { _ ⇒
        println("okay that's all")
        ExitCode.Success
      }

}
