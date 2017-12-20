/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

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
