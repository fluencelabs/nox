package fluence.node.workers.subscription

import io.circe.{Decoder, HCursor}

/**
 * Represents response code for broadcastTxSync.
 *
 */
case class TxResponseCode(code: Int, info: Option[String])

object TxResponseCode {
  implicit val decodeTxResponseCode: Decoder[TxResponseCode] = new Decoder[TxResponseCode] {
    final def apply(c: HCursor): Decoder.Result[TxResponseCode] =
      for {
        code <- c.downField("result").downField("code").as[Int]
        info <- c.downField("result").downField("info").as[Option[String]]
      } yield {
        new TxResponseCode(code, info)
      }
  }
}

/**
 * Represents response code for ABCI_query.
 *
 */
case class QueryResponseCode(code: Int, info: Option[String])

object QueryResponseCode {
  implicit val decodeQueryResponseCode: Decoder[QueryResponseCode] = new Decoder[QueryResponseCode] {
    final def apply(c: HCursor): Decoder.Result[QueryResponseCode] =
      for {
        code <- c.downField("result").downField("response").downField("code").as[Int]
        info <- c.downField("result").downField("response").downField("info").as[Option[String]]
      } yield {
        new QueryResponseCode(code, info)
      }
  }
}
