package fluence.client.cli

import cats.InjectK
import cats.free.Free.inject
import cats.free.Free

import scala.language.higherKinds

class CliOps[F[_]](implicit I: InjectK[CliOp, F]) {

  def exit: Free[F, Unit] = inject[CliOp, F](CliOp.Exit)

  def get(key: String): Free[F, Unit] = inject[CliOp, F](CliOp.Get(key))

  def put(key: String, value: String): Free[F, Unit] = inject[CliOp, F](CliOp.Put(key, value))

  def readLine(prefix: String): Free[F, String] = inject[CliOp, F](CliOp.ReadLine(prefix))

  def readPassword(prefix: String): Free[F, String] = inject[CliOp, F](CliOp.ReadLine(prefix))

  def print(lines: String*): Free[F, Unit] = inject[CliOp, F](CliOp.PrintLines(lines))
}
