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
  // Description for reserved non-VM function call that explicitly closes sessions by the client.
  val CloseSession = FunctionCallDescription(None, "@closeSession", Nil)

  private val FunctionWithModuleAndArgListPattern: Regex = "(\\w+)\\.(\\w+)\\(([\\w,]*)\\)".r
  private val FunctionWithoutModuleAndArgListPattern: Regex = "([\\w\\@]+)\\(([\\w,]*)\\)".r
  private val NonEmptyArgListPattern: Regex = "\\w+(,\\w+)*".r

  /**
   * Parses text payload in `[moduleName].functionName(arg1, ..., argN)` format to a typed function call description.
   *
   * @param payload text representation of the function invocation
   */
  def parse[F[_]](payload: String)(implicit F: Monad[F]): EitherT[F, StateMachineError, FunctionCallDescription] =
    for {
      parsedPayload <- payload match {
        case FunctionWithModuleAndArgListPattern(module, functionName, uncheckedArgList) =>
          EitherT.rightT((Some(module), functionName, uncheckedArgList))
        case FunctionWithoutModuleAndArgListPattern(functionName, uncheckedArgList) =>
          EitherT.rightT((None, functionName, uncheckedArgList))
        case _ => EitherT.leftT(wrongPayloadFormatError(payload))
      }
      (module, functionName, unparsedArgList: String) = parsedPayload

      parsedArgListET: EitherT[F, StateMachineError, List[String]] = unparsedArgList match {
        case NonEmptyArgListPattern(_) => EitherT.rightT(unparsedArgList.split(",").toList)
        case "" => EitherT.rightT(Nil)
        case _ => EitherT.leftT(wrongPayloadArgumentListFormatError(unparsedArgList))
      }
      parsedArgList <- parsedArgListET
    } yield FunctionCallDescription(module, functionName, parsedArgList)

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
