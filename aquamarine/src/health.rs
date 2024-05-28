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

use health::HealthCheck;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct VMPoolHealth {
    expected_count: usize,
    current_count: Arc<AtomicUsize>,
}

impl VMPoolHealth {
    pub fn new(expected_count: usize) -> Self {
        Self {
            expected_count,
            current_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn increment_count(&self) {
        self.current_count.fetch_add(1, Ordering::Release);
    }
}

impl HealthCheck for VMPoolHealth {
    fn status(&self) -> eyre::Result<()> {
        let current = self.current_count.load(Ordering::Acquire);
        if self.expected_count != current {
            return Err(eyre::eyre!(
                "VM pool isn't full. Current: {}, Expected: {}",
                current,
                self.expected_count
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::VMPoolHealth;
    use std::thread;

    #[test]
    fn test_vm_pool_health_empty() {
        let pool_health = VMPoolHealth::new(0);
        let status = pool_health.status();
        assert!(status.is_ok());
    }

    #[test]
    fn test_vm_pool_health_partial() {
        let pool_health = VMPoolHealth::new(5);
        pool_health.increment_count();
        pool_health.increment_count();
        pool_health.increment_count();

        let status = pool_health.status();
        assert!(status.is_err());
    }

    #[test]
    fn test_vm_pool_health_full() {
        let pool_health = VMPoolHealth::new(3);
        pool_health.increment_count();
        pool_health.increment_count();
        pool_health.increment_count();

        let status = pool_health.status();
        assert!(status.is_ok());
    }

    #[test]
    fn test_vm_pool_health_concurrent_access() {
        let pool_health = VMPoolHealth::new(100);
        let mut handles = vec![];

        // Simulate concurrent access by spawning multiple threads.
        for _ in 0..50 {
            let pool_health_clone = pool_health.clone();
            let handle = thread::spawn(move || {
                for _ in 0..2 {
                    pool_health_clone.increment_count();
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to finish.
        for handle in handles {
            handle.join().unwrap();
        }

        let status = pool_health.status();
        assert!(status.is_ok());
    }
}
