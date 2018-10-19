package fluence.statemachine.util
import java.util.concurrent.TimeUnit

/**
 * Value class to measure time period's duration.
 *
 * @param startNanos nanoseconds at the period start
 */
class TimeMeter(val startNanos: Long) extends AnyVal {

  /**
   * Returns a non-negative number which is the amount of milliseconds from [[TimeMeter]] initialization to now.
   */
  def millisElapsed: Long = TimeUnit.MILLISECONDS.convert(System.nanoTime() - startNanos, TimeUnit.NANOSECONDS)

  /**
   * Returns a number which is the amount of milliseconds from given value in past to [[TimeMeter]] initialization.
   *
   * @param millisInPast milliseconds timestamp measured at some instant in past
   */
  def millisFromPastToStart(millisInPast: Long): Long =
    TimeUnit.MILLISECONDS.convert(startNanos, TimeUnit.NANOSECONDS) - millisInPast
}

object TimeMeter {
  def apply(): TimeMeter = new TimeMeter(System.nanoTime())
}
