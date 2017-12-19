package fluence.dataset.allocate

import scala.language.higherKinds

/**
 * Remotely-accessible interface to negotiate allocation of a dataset contract
 * @tparam F Effect
 * @tparam C Contract
 */
trait ContractAllocatorRPC[F[_], C] {
  /**
   * Offer a contract. Node should check and preallocate required resources, save offer, and sign it
   *
   * @param contract A blank contract
   * @return Signed contract, or F is an error
   */
  def offer(contract: C): F[C]

  /**
   * Allocate dataset: store the contract, create storage structures, form cluster
   *
   * @param contract A sealed contract with all nodes and client signatures
   * @return Allocated contract
   */
  def allocate(contract: C): F[C]

}
