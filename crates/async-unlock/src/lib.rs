/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

//! This crate describes functions to work with async_std's Mutex in a manner that guarantees
//! shortest possible lock time by dropping MutexGuard ASAP.

use std::future::Future;
use std::ops::DerefMut;
use tokio::sync::Mutex;

/// Performs async computation on a value inside Mutex
/// This function guarantees that Mutex will be unlocked before awaiting computation described by `f`
pub async fn unlock_f<T, R, F: Future<Output = R>>(m: &Mutex<T>, f: impl FnOnce(&mut T) -> F) -> R {
    unlock(m, f).await.await
}

/// Performs computation on a value inside Mutex, unlocking Mutex immediately after `f` is computed
pub async fn unlock<T, R>(m: &Mutex<T>, f: impl FnOnce(&mut T) -> R) -> R {
    let mut guard = m.lock().await;
    let result = f(guard.deref_mut());
    drop(guard);
    result
}
