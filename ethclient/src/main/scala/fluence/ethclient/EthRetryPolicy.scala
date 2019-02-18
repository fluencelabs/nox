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

package fluence.ethclient
import scala.concurrent.duration._

/**
 * Exponential backoff delays.
 *
 * @param delayPeriod will be applied next time
 * @param maxDelay upper bound for a single delay
 */
case class EthRetryPolicy(delayPeriod: FiniteDuration, maxDelay: FiniteDuration) {

  /**
   * Next retry policy with delayPeriod multiplied times two, if maxDelay is not yet reached
   */
  def next: EthRetryPolicy =
    if (delayPeriod == maxDelay) this
    else {
      val nextDelay = delayPeriod * 2
      if (nextDelay > maxDelay) copy(delayPeriod = maxDelay) else copy(delayPeriod = nextDelay)
    }
}

object EthRetryPolicy {
  val Default = EthRetryPolicy(1.second, 1.minute)
}
