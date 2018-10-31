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

package fluence.statemachine.util
import io.prometheus.client.{CollectorRegistry, Counter}

/**
 * Provides various actions for Prometheus metric collection.
 */
object Metrics {

  /**
   * Registers a new Prometheus counter collector.
   *
   * @param name collector name
   */
  def registerCounter(name: String): Counter =
    Counter
      .build()
      .name(name)
      .help(name)
      .register()

  /**
   * Registers a new Prometheus counter collector.
   *
   * @param name collector name
   * @param labels collector label set
   */
  def registerCounter(name: String, labels: String*): Counter =
    Counter
      .build()
      .name(name)
      .help(name)
      .labelNames(labels: _*)
      .register()

  /**
   * Clears all previously registered Prometheus metric collectors.
   */
  def resetCollectors(): Unit = CollectorRegistry.defaultRegistry.clear()
}
