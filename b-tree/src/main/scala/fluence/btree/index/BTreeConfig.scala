package fluence.btree.index

/**
 *
 * @param arity maximum size of node (max number of node children)
 * @param alpha minimum capacity factor of node. Should be between 0 and 0.5.
 *               0.25 means that each node except root should contains between 25% and 100% children.
 */
case class BTreeConfig(
  arity: Int = 8,
  alpha: Float = 0.25F
) {

}

object BTreeConfig {

  // todo read config from file

}
