package fluence.btree.server.network

import fluence.btree.client.network.{ BTreeServerResponse, InitClientRequest, _ }
import fluence.btree.server.network.BTreeServerNetwork.RequestHandler

/**
 * Base network abstraction for interaction BTree server with the BTree client.
 *
 * @tparam F The type of effect, box for returning value
 */
trait BTreeServerNetwork[F[_]] {

  /**
   * Handler for init request [[fluence.btree.client.network.InitClientRequest]]
   * from [[fluence.btree.client.MerkleBTreeClient]] contains all business logic
   * for processing client init requests.
   */
  val initRequestHandler: RequestHandler[F]

  /** Start the BTreeServerNetwork. */
  def start(): F[Boolean]

  /** Shut the BTreeServerNetwork down. */
  def shutdown(): F[Boolean]

  // todo implement with transport, this is pseudocode for example!

  //  def get(requests: Observable[BTreeClientRequest]): Observable[BTreeServerResponse] = {
  //    requests.getNext().flatMap {
  //      case initReq: InitClientRequest =>
  //        val result = Observable.new[BTreeServerResponse]
  //
  //      val askClientFn: BTreeServerResponse ⇒ F[BTreeClientRequest] =
  //        response ⇒ {
  //          result.push(GRPC.CmpCommand(response)).flatMap { _ =>
  //            requests.getNext().as[F[BTreeClientRequest]]
  //          }
  //        }
  //
  //      initRequestHandler(initReq, askClientFn)
  //    }
  //    ...
  //  }

}

object BTreeServerNetwork {

  type RequestHandler[F[_]] = (InitClientRequest, BTreeServerResponse ⇒ F[Option[BTreeClientRequest]]) ⇒ F[Unit]

}
