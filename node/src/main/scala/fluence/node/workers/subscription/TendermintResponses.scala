package fluence.node.workers.subscription

import io.circe.{Decoder, HCursor}

/**
 * Represents response code for broadcastTxSync.
 *
 */
case class TxResponseCode(code: Int)

object TxResponseCode {
  implicit val decodeTxResponseCode: Decoder[TxResponseCode] = new Decoder[TxResponseCode] {
    final def apply(c: HCursor): Decoder.Result[TxResponseCode] =
      for {
        code <- c.downField("result").downField("code").as[Int]
      } yield {
        new TxResponseCode(code)
      }
  }
}

/**
 * Represents response code for ABCI_query.
 *
 */
case class QueryResponseCode(code: Int)

object QueryResponseCode {
  implicit val decodeQueryResponseCode: Decoder[QueryResponseCode] = new Decoder[QueryResponseCode] {
    final def apply(c: HCursor): Decoder.Result[QueryResponseCode] =
      for {
        code <- c.downField("result").downField("response").downField("code").as[Int]
      } yield {
        new QueryResponseCode(code)
      }
  }
}
