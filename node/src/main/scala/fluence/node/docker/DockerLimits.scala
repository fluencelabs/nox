package fluence.node.docker

/**
 * Limits on cpu and memory of a container
 * @see [[DockerParams.cpus]], [[DockerParams.memory]], [[DockerParams.memoryReservation]]
 *
 * @param cpus Fraction number of max cores available to a container.
 * @param memoryMb A hard limit on maximum amount of memory available to a container
 * @param memoryReservationMb Amount of memory guaranteed to be allocated for a container
 */
case class DockerLimits(cpus: Option[Double], memoryMb: Option[Int], memoryReservationMb: Option[Int])
