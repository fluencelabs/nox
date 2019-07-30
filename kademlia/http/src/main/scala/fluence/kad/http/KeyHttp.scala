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

package fluence.kad.http

import cats.Id
import cats.data.{NonEmptyList, Validated, ValidatedNel}
import fluence.kad.protocol.Key
import org.http4s.{ParseFailure, QueryParamDecoder, QueryParameterValue}

private[http] object KeyHttp {

  implicit object KeyDecoder extends QueryParamDecoder[Key] {
    override def decode(value: QueryParameterValue): ValidatedNel[ParseFailure, Key] =
      Validated
        .fromEither(
          Key.fromB58[Id](value.value).value
        )
        .leftMap(err ⇒ NonEmptyList.one(ParseFailure(err.message, "Key codec failure")))
  }

  object KeyVar {

    def unapply(str: String): Option[Key] =
      Key.fromB58[Id](str).value.toOption
  }

}
