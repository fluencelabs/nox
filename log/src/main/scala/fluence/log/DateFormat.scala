package fluence.log

import java.util.Date

/**
 * JS and JVM have work\ different with date formatting.
 */
trait DateFormat {

  /**
   * Formats date to string.
   */
  def format(date: Date): String
}
