mod counter;

/// Public function for export

static mut COUNTER_: counter::Counter = counter::Counter { counter: 0 };

#[no_mangle]
pub unsafe fn inc() {
    COUNTER_.inc()
}

#[no_mangle]
pub unsafe fn get() -> i64 {
    COUNTER_.get()
}

fn main() {
    // do nothing
}