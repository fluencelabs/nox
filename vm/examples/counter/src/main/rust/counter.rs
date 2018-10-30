/// Counter implementation.

pub struct Counter {
    pub counter: i64,
}

impl Counter {
    pub fn inc(&mut self) {
        self.counter += 1
    }

    pub fn get(&self) -> i64 {
        self.counter
    }
}
