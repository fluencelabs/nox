package fluence.btree.server.network.commands

import cats.MonadError
import cats.syntax.functor._
import fluence.btree.client.merkle.{ MerklePath, MerkleRootCalculator }
import fluence.btree.client.network._
import fluence.btree.client.{ Key, Value }
import fluence.btree.server.Leaf
import fluence.btree.server.core._

/**
 * Command for putting key and value to the BTree.
 *
 * @param askRequiredDetails A function that ask client to give some required details for the next step
 * @tparam F The type of effect, box for returning value
 */
case class PutCommandImpl[F[_]](
    merkleRootCalculator: MerkleRootCalculator,
    askRequiredDetails: (BTreeServerResponse) ⇒ F[Option[BTreeClientRequest]]
)(implicit ME: MonadError[F, Throwable]) extends BaseCommand[F](askRequiredDetails) with PutCommand[F, Key, Value] {

  def putDetails(leaf: Option[Leaf]): F[PutDetails] = {
    val response =
      leaf.map(l ⇒ LeafResponse(l.keys, l.values))
        .getOrElse(LeafResponse(Array.empty[Key], Array.empty[Value]))

    askRequiredDetails(response)
      .map { case Some(PutRequest(key, value, searchResult)) ⇒ PutDetails(key, value, searchResult) }
  }

  override def submitMerklePath(merklePath: MerklePath, wasSplitting: Boolean): F[Unit] = {
    val response = if (wasSplitting)
      VerifyPutWithRebalancingResponse(merklePath)
    else
      VerifySimplePutResponse(merkleRootCalculator.calcMerkleRoot(merklePath))

    askRequiredDetails(response)
      .map(_ ⇒ ())
  }
}
