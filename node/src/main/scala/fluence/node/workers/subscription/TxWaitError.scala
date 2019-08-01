package fluence.node.workers.subscription

import fluence.effects.tendermint.rpc.http.RpcError

/**
 * Errors for `txAwait` API
 */
trait TxAwaitError
case class TendermintResponseError(responseError: String) extends TxAwaitError
case class RpcTxAwaitError(rpcError: RpcError) extends TxAwaitError
case class TxParsingError(msg: String, tx: String) extends TxAwaitError
case class TxInvalidError(msg: String) extends TxAwaitError
