package fluence.btree.server.network.commands

import cats.MonadError
import cats.syntax.functor._
import fluence.btree.client.network._
import fluence.btree.client.{ Key, Value }
import fluence.btree.server.core.{ GetCommand, LeafNode }

/**
 * Command for searching some value in BTree (by client search key).
 * Search key is stored at the client. BTree server will never know search key.
 *
 * @param askRequiredDetails A function that ask client to give some required details for the next step
 *
 * @tparam F The type of effect, box for returning value
 */
class GetCommandImpl[F[_]](
    askRequiredDetails: (BTreeServerResponse) ⇒ F[Option[BTreeClientRequest]]
)(implicit ME: MonadError[F, Throwable]) extends BaseCommand[F](askRequiredDetails) with GetCommand[F, Key, Value] {

  override def submitLeaf(leaf: Option[LeafNode[Key, Value]]): F[Unit] = {
    val response =
      leaf.map(l ⇒ LeafResponse(l.keys, l.values))
        .getOrElse(LeafResponse(Array.empty[Key], Array.empty[Value]))

    askRequiredDetails(response)
      .map(_ ⇒ ())
  }

}
