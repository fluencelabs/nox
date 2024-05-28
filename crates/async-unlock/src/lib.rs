/*
 * Copyright 2024 Fluence DAO
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
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
