package fluence.dataset.allocate

import java.time.Instant

/**
 * Wrapper for internal node's contracts cache with lastUpdated timestamp to invalidate caches
 *
 * @tparam C contract's type
 */
private[dataset] case class ContractRecord[C](
    contract: C,
    lastUpdated: Instant = Instant.now())
