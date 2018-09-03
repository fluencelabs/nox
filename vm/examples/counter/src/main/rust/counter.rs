
/// Counter implementation.

pub struct Counter {
    counter: i64
}

impl Counter {

    pub fn inc(&mut self) {
        self.counter += 1
    }

    pub fn get(&self) -> i64 {
        self.counter
    }
}

/// Public function for export

static mut COUNTER_: Counter = Counter { counter: 0 };

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