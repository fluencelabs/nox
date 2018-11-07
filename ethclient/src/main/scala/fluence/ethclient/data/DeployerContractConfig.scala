package fluence.ethclient.data

/**
 * Deployer contract settings.
 *
 * @param deployerContractOwnerAccount contract owner eth account
 * @param deployerContractAddress address, where contract is deployed
 */
case class DeployerContractConfig(deployerContractOwnerAccount: String, deployerContractAddress: String)
