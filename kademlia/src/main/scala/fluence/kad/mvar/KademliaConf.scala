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

package fluence.kad.mvar

import scala.concurrent.duration.Duration
import scala.language.higherKinds

/**
 *
 * @param maxSiblingsSize Maximum number of siblings to store, e.g. K * Alpha
 * @param maxBucketSize   Maximum size of a bucket, usually K
 * @param parallelism     Parallelism factor (named Alpha in paper)
 * @param pingExpiresIn   Duration to avoid too frequent ping requests, used in [[fluence.kad.Bucket.update]]
 */
case class KademliaConf(
  maxBucketSize: Int,
  maxSiblingsSize: Int,
  parallelism: Int,
  pingExpiresIn: Duration
)
