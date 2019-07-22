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

package fluence.kad.conf

import fluence.kad.state.Bucket
import fluence.kad.{routing, state}

import scala.concurrent.duration.Duration

/**
 *
 * @param maxSiblingsSize Maximum number of siblings to store, e.g. K * Alpha
 * @param maxBucketSize   Maximum size of a bucket, usually K
 * @param parallelism     Parallelism factor (named Alpha in paper)
 * @param pingExpiresIn   Duration to avoid too frequent ping requests, used in [[Bucket.update]]
 * @param refreshing Configuration for Refreshing extension, see [[routing.RefreshingIterativeRouting]]
 * @param store Configuration for Kademlia's persistent contacts store, see [[state.StoredRoutingState]]
 */
case class RoutingConf(
  maxBucketSize: Int,
  maxSiblingsSize: Int,
  parallelism: Int,
  pingExpiresIn: Duration,
  refreshing: Option[RoutingConf.Refreshing] = None,
  store: Option[String] = None
)

object RoutingConf {

  /**
   * @param period Each bucket's refreshing period
   * @param neighbors How many neighbors to lookup for the bucket refresh
   */
  case class Refreshing(period: Duration, neighbors: Int)
}
