package fluence.btree.server.network.commands

import cats.MonadError
import cats.syntax.functor._
import fluence.btree.client.Key
import fluence.btree.client.network.{ BTreeClientRequest, BTreeServerResponse, NextChildSearchResponse, ResumeSearchRequest }
import fluence.btree.server.core.{ BranchNode, TreeCommand }

/**
 * Abstract command implementation for searching some value in BTree (by client search key).
 * Search key is stored at the client. BTree server will never know search key.
 *
 * @param askRequiredDetails A function that ask client to give some required details for the next step
 *
 * @param ME Monad error
 * @tparam F The type of effect, box for returning value
 */
abstract class BaseCommand[F[_]](
    askRequiredDetails: (BTreeServerResponse) ⇒ F[Option[BTreeClientRequest]]
)(implicit ME: MonadError[F, Throwable]) extends TreeCommand[F, Key] {

  override def nextChildIndex(branch: BranchNode[Key, _]): F[Int] = {
    askRequiredDetails(NextChildSearchResponse(branch.keys, branch.childsChecksums))
      .map { case Some(ResumeSearchRequest(nextChildIdx)) ⇒ nextChildIdx }

  }
}
