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

package fluence.statemachine.tree
import fluence.statemachine.StoreKey

/**
 * Structure denoting some absolute or relative path in the tree of [[TreeNode]]-s.
 *
 * @tparam T type used to store a single component in the path
 */
sealed trait TreePath[T]

object TreePath {

  /**
   * Recursively builds the path from given components.
   *
   * @param components path hierarchy' components describing path from the root to the target key
   * @tparam T type used to store a single component in the path
   */
  def apply[T](components: List[T]): TreePath[T] = components match {
    case Nil => EmptyTreePath()
    case next :: rest => SplittableTreePath(next, TreePath[T](rest))
  }

  /**
   * Parses a path from a string representation.
   *
   * @param pathString '/'-separated string representation
   * @return either parsed path or error message
   */
  def parse(pathString: String): Either[String, TreePath[StoreKey]] = {
    val splitPath = pathString.split('/').toList
    Either.cond(!pathString.endsWith("/") && !splitPath.contains(""), TreePath(splitPath), "Invalid query path")
  }

}

/**
 * Splittable, non-empty path.
 *
 * @param next the first component
 * @param rest the rest of components
 * @tparam T type used to store a single component in the path
 */
case class SplittableTreePath[T](next: T, rest: TreePath[T]) extends TreePath[T]

/**
 * Empty path.
 * Corresponds to the root for absolute paths.
 * Corresponds to the current node for relativa paths.
 *
 * @tparam T type used to store a single component in the path
 */
case class EmptyTreePath[T]() extends TreePath[T]
