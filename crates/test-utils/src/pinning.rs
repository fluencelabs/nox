use cpu_utils::pinning::ThreadPinner;
use cpu_utils::LogicalCoreId;

pub const DUMMY: DummyThreadPinner = DummyThreadPinner {};

pub struct DummyThreadPinner;

impl ThreadPinner for DummyThreadPinner {
    #[inline]
    fn pin_current_thread_to(&self, _core_id: LogicalCoreId) -> bool {
        true
    }

    #[inline]
    fn pin_current_thread_to_cpuset(&self, _core_ids: &[LogicalCoreId]) -> bool {
        true
    }
}
