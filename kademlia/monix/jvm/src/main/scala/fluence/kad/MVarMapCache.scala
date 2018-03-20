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

package fluence.kad

import cats.data.StateT
import monix.eval.{MVar, Task}

import scala.collection.concurrent.TrieMap

//todo move this to some utility module, replace all similar caches with MVarCache
//todo we always return non-default parameters, prove it on types
/**
 * WIP
 */
class MVarMapCache[K, V](default: V) {
  //todo maybe store Option[V] and add MVar(None) for default as values, on `get` return None if None in MVar (flatten)
  private val writeState = TrieMap.empty[K, Task[MVar[V]]]
  private val readState = TrieMap.empty[K, V]

  private def getMVar(key: K, value: V): Task[MVar[V]] =
    writeState.getOrElseUpdate(key, MVar(value).memoize)

  import RunOnMVar.runOnMVar

  def update(key: K, value: V): Task[Unit] =
    for {
      s <- getMVar(key, value)
      res <- runOnMVar(
        s,
        StateT.set(value),
        readState.update(key, _: V)
      )
    } yield res

  def get(key: K): Option[V] =
    readState.get(key)

  def getOrAdd(key: K, value: V): Task[V] =
    for {
      s <- getMVar(key, value)
      res <- runOnMVar(
        s,
        StateT.get[Task, V],
        readState.update(key, _: V)
      )
    } yield res

  def getOrAddF(key: K, value: ⇒ Task[V]): Task[V] =
    for {
      s <- getMVar(key, default)
      res <- runOnMVar(
        s,
        StateT.modifyF[Task, V](a ⇒ if (a != default) Task.pure(a) else value).get,
        readState.update(key, _: V)
      )
    } yield res

  def modify(key: K, modify: V ⇒ V): Task[Boolean] =
    writeState.get(key) match {
      case None ⇒ Task.pure(false)
      case Some(value) ⇒
        for {
          v <- value
          res <- runOnMVar(
            v,
            StateT.modify[Task, V](modify).map(_ ⇒ true),
            readState.update(key, _: V)
          )
        } yield res
    }

  def modifyValueOrDefault(key: K, modify: V ⇒ V): Task[Boolean] =
    for {
      s <- getMVar(key, default)
      res <- runOnMVar(
        s,
        StateT.modify[Task, V](modify).map(_ ⇒ true),
        readState.update(key, _: V)
      )
    } yield res

}
