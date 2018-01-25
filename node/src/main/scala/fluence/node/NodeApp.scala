/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.node

import cats.syntax.show._
import fluence.crypto.keypair.KeyPair
import fluence.kad.protocol.{ Contact, Key }
import monix.eval.Coeval
import monix.execution.Scheduler.Implicits.global
import org.slf4j.LoggerFactory

import scala.io.StdIn
import scala.util.{ Failure, Success }

object NodeApp extends App {

  val log = LoggerFactory.getLogger(getClass)

  // For demo purposes
  val keySeed = StdIn.readLine(Console.CYAN + "Who are you?\n> " + Console.RESET).getBytes()
  val keyPair = KeyPair.fromBytes(keySeed, keySeed)

  val nodeComposer = new NodeComposer(keyPair)

  import nodeComposer.{ kad, server }

  server.start().attempt.runAsync.foreach {
    case Left(err) ⇒
      log.error("Can't launch server", err)

    case Right(_) ⇒
      log.info("Server launched")

      server.contact.foreach { contact ⇒
        println("Your contact is: " + contact.show)
        println("You may share this seed for others to join you: " + Console.MAGENTA + contact.b64seed + Console.RESET)
      }
  }

  def cmd(s: String): Unit = println(Console.BLUE + s + Console.RESET)

  while (true) {

    cmd("join(j) / lookup(l)")

    StdIn.readLine() match {
      case "j" | "join" ⇒
        cmd("join seed?")
        val p = StdIn.readLine()
        Coeval.fromEval(
          Contact.readB64seed(p)
        ).memoize.attempt.value match {
            case Left(err) ⇒
              log.error("Can't read the seed", err)

            case Right(c) ⇒
              kad.join(Seq(c), 16).runAsync.onComplete {
                case Success(_) ⇒ cmd("ok")
                case Failure(e) ⇒
                  log.error("Can't join", e)
              }
          }

      case "l" | "lookup" ⇒
        cmd("lookup myself")
        kad.handleRPC
          .lookup(Key.XorDistanceMonoid.empty, 10)
          .map(_.map(_.show).mkString("\n"))
          .map(println)
          .runAsync.onComplete {
            case Success(_) ⇒ println("ok")
            case Failure(e) ⇒
              log.error("Can't lookup", e)
          }

      case "s" | "seed" ⇒
        server.contact.map(_.b64seed).runAsync.foreach(cmd)

      case "q" | "quit" | "x" | "exit" ⇒
        cmd("exit")
        System.exit(0)

      case _ ⇒
    }
  }
}
