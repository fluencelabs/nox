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

package fluence.statemachine.tx
import cats.Monad
import cats.data.EitherT
import cats.syntax.either._
import fluence.statemachine.error.{PayloadParseError, StateMachineError}

import scala.language.higherKinds
import scala.util.matching.Regex

/**
 * Description of a function invocation with concrete arguments.
 *
 * @param module VM module containing the invoked function
 * @param functionName name of the invoked function
 * @param argList ordered sequence of function arguments
 */
case class FunctionCallDescription(module: Option[String], functionName: String, argList: List[String])

object FunctionCallDescription {

  /**
   * Description for reserved non-VM function call that explicitly closes sessions by the client.
   */
  val CloseSession = FunctionCallDescription(None, "@closeSession", Nil)

  // ^ start of the line, needed to capture whole string, not just substring
  // (\w+(?=\.))* optional module name, must be followed by dot. dot isn't captured. ?= is called lookahead.
  // (@?\w+) function name, optionally prefixed by @
  // \((.*?)\) anything inside parentheses, will be parsed later by argRx
  // $ end of the line, needed to capture whole string, not just substring
  private val payloadPattern = """^((\w+)\.)?(@?\w+)\((.*?)\)$""".r

  // anything but the quotes inside quotes OR any number with dots
  private val argsPattern = """("[^"]*"|[\d.]+)""".r

  /**
   * Parses text payload in `[moduleName].functionName(arg1, ..., argN)` format to a typed function call description.
   *
   * @param payload text representation of the function invocation
   */
  def parse[F[_]](payload: String)(implicit F: Monad[F]): EitherT[F, StateMachineError, FunctionCallDescription] =
    EitherT.fromEither(for {
      parsedPayload <- payload match {
        case payloadPattern(_, m, f, args) => Either.right((Option(m), f, args))
        case _ => Either.left(wrongPayloadFormatError(payload))
      }
      (module, functionName, unparsedArgList) = parsedPayload
      parsedArgList <- {
        if (unparsedArgList.isEmpty) Either.right(List.empty)
        else {
          argsPattern.findAllMatchIn(unparsedArgList).toList.map(_.group(1)) match {
            case Nil => Either.left(wrongPayloadArgumentListFormatError(unparsedArgList))
            case l => Either.right(l)
          }
        }

      }
    } yield FunctionCallDescription(module, functionName, parsedArgList))

  /**
   * Produces [[StateMachineError]] corresponding to payload that cannot be parsed to a function call.
   *
   * @param payload wrong payload
   */
  private def wrongPayloadFormatError(payload: String): StateMachineError =
    PayloadParseError("WrongPayloadFormat", s"Wrong payload format: $payload")

  /**
   * Produces [[StateMachineError]] corresponding to payload's argument list that cannot be parsed to
   * correct function arguments.
   *
   * @param unparsedArgList wrong payload argument list
   */
  private def wrongPayloadArgumentListFormatError(unparsedArgList: String): StateMachineError =
    PayloadParseError("WrongPayloadArgumentListFormat", s"Wrong payload arguments: $unparsedArgList")
}
