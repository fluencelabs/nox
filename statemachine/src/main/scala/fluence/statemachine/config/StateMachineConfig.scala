package fluence.statemachine.config

/**
 * State machine settings.
 *
 * @param sessionExpirationPeriod Period after which the session becomes expired.
 *                                Measured as difference between the current `txCounter` value and
 *                                its value at the last activity in the session.
 */
case class StateMachineConfig(sessionExpirationPeriod: Long)
