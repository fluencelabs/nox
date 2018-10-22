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
