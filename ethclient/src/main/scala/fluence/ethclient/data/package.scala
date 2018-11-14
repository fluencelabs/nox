package fluence.ethclient
import org.web3j.abi.datatypes.DynamicArray
import org.web3j.abi.datatypes.generated.{Bytes24, Bytes32, Uint16, Uint256}
import org.web3j.tuples.generated

package object data {

  /**
   * Corresponds to return type for Deployer.getCluster method.
   */
  type ContractClusterTuple = generated.Tuple6[
    Bytes32,
    Bytes32,
    Uint256,
    DynamicArray[Bytes32],
    DynamicArray[Bytes24],
    DynamicArray[Uint16]
  ]
}
