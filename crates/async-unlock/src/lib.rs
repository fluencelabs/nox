//! This crate describes functions to work with async_std's Mutex in a manner that guarantees
//! shortest possible lock time by dropping MutexGuard ASAP.

use async_std::sync::Mutex;
use std::future::Future;
use std::ops::DerefMut;

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
