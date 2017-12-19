package fluence.dataset.allocate

import fluence.kad.Key
import scala.language.higherKinds

/**
 * RPC for node's local cache for contracts
 *
 * @tparam F Effect
 * @tparam C Contract
 */
trait ContractsCacheRPC[F[_], C] {

  /**
   * Tries to find a contract in local cache
   *
   * @param id Dataset ID
   * @return Optional locally found contract
   */
  def find(id: Key): F[Option[C]]

  /**
   * Ask node to cache the contract
   *
   * @param contract Contract to cache
   * @return If the contract is cached or not
   */
  def cache(contract: C): F[Boolean]

}
