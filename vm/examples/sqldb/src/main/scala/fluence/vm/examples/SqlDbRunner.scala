/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.vm.examples

import cats.Monad
import cats.data.EitherT
import cats.effect.{ExitCode, IO, IOApp}
import fluence.vm.VmError.InternalVmError
import fluence.vm.{VmError, WasmVm}

import scala.language.higherKinds

object SqlDbRunner extends IOApp {

  override def run(args: List[String]): IO[ExitCode] = {

    val program = for {
      inputFile ← EitherT(getInputFile(args).attempt)
        .leftMap(e ⇒ InternalVmError(e.getMessage, Some(e)))
      vm ← WasmVm[IO](Seq(inputFile))
      initState ← vm.getVmState[IO]

      // select * from Quotations;
      _ ← vm.invoke[IO](None, "set_query_wildcard") // invoke WASM select
      _ ← showResult("select * from Quotations;", vm) // grub results with cursor

      // select * from Quotations where price = 6700;
      _ ← vm.invoke[IO](None, "set_query_wildcard_where", Seq("2", "6700"))
      _ ← showResult("select * from Quotations where price = 6700;", vm)

      // select count(*) from Quotations;
      _ ← vm.invoke[IO](None, "set_count_query")
      _ ← showResult("select count(*) from Quotations;", vm)

      // select count(*) from Quotations where symbol = 2;
      _ ← vm.invoke[IO](None, "set_count_query_where", Seq("1", "2"))
      _ ← showResult("select count(*) from Quotations where symbol = 2;", vm)

      // select avg(price) from Quotations;
      _ ← vm.invoke[IO](None, "set_average_query", Seq("2"))
      _ ← showResult("select avg(price) from Quotations;", vm)

      // select avg(price) from Quotations where symbol = 2;
      _ ← vm.invoke[IO](None, "set_average_query_where", Seq("2", "1", "2"))
      _ ← showResult("select avg(price) from Quotations where symbol = 2;", vm)

      finishState ← vm.getVmState[IO].toVmError
    } yield {
      s"[SUCCESS] Execution Results.\n" +
        s"initState=$initState \n" +
        s"finishState=$finishState"
    }

    program.value.map {
      case Left(err) ⇒
        println(s"[Error]: $err cause=${err.getCause}")
        ExitCode.Error
      case Right(value) ⇒
        println(value)
        ExitCode.Success
    }
  }

  private def getInputFile(args: List[String]): IO[String] = IO {
    args.headOption match {
      case Some(value) ⇒
        println(s"Starts for input file $value")
        value
      case None ⇒
        throw new IllegalArgumentException("Full path for counter.wasm is required!")
    }
  }

  /** fetch all elements from all rows recursively and stack safe */
  private def getResult(vm: WasmVm): EitherT[IO, VmError, Seq[Seq[String]]] = {

    type M[A] = EitherT[IO, VmError, A]
    type RightType = Seq[Seq[String]]
    type LeftType = Option[(String, RightType)]

    vm.invoke[IO](None, "next_row").flatMap { row ⇒
      Monad[M].tailRecM(row.map(_.toString → Seq[Seq[String]]())) {
        case Some(("-1", acc)) ⇒
          // there is no more rows for this query, stop and return result
          Monad[M].pure(Right[LeftType, RightType](acc)): M[Either[LeftType, RightType]]
        case Some((_, acc)) ⇒
          // Fetch all elems for current row, add to results and check next one

          getAllRowElems(vm).flatMap { elems ⇒
            vm.invoke[IO](None, "next_row").toVmError.map { elem ⇒
              val elemAsStr = elem.map(_.toString).get
              Left(Some(elemAsStr → (acc ++ Seq(elems))))
            }
          }: M[Either[LeftType, RightType]]
      }
    }

  }

  /** fetch all elements for current row recursively and stack safe */
  private def getAllRowElems(vm: WasmVm): EitherT[IO, VmError, Seq[String]] = {

    type M[A] = EitherT[IO, VmError, A]
    type LeftType = Option[(String, Seq[String])]
    type RightType = Seq[String]

    vm.invoke[IO](None, "next_field").flatMap { elem ⇒
      Monad[M].tailRecM(elem.map(_.toString → Seq[String]())) {
        case Some(("-1", acc)) ⇒
          // there is no more elements in current row, stop and return result
          Monad[M].pure(Right[LeftType, RightType](acc))
        case Some((elemId, acc)) ⇒
          // There is a next element, add to results and check next one

          vm.invoke[IO](None, "next_field").toVmError.map { elem ⇒
            val elemAsStr = elem.map(_.toString).get
            Left(Some(elemAsStr → (acc ++ Seq(elemId))))
          }: M[Either[LeftType, RightType]]
      }
    }
  }

  /** Fetches result and prints it to stdout */
  private def showResult(query: String, vm: WasmVm): EitherT[IO, VmError, Unit] = {
    getResult(vm).flatMap { res ⇒
      EitherT.right(IO(println(s"> $query \n${prettyPrint(res)}\n")))
    }
  }

  /** Converts table to pretty string */
  private def prettyPrint(table: Seq[Seq[String]]): String =
    table
      .map(row ⇒ row.map(el ⇒ f"$el%6s").mkString("|", "|", "|"))
      .mkString("\n")

}
