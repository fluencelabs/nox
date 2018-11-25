mod settings;
use settings::{ITERATION_COUNT, FIB_NUMBER};

#[no_mangle]
pub extern "C" fn fib(num: i64) -> i64 {
	if num == 1 || num == 2 {
		return 1
	}

	fib(num - 1) * fib(num - 2)
}

#[no_mangle]
pub extern "C" fn test() -> i64 {
	for _ in 1..ITERATION_COUNT {
		fib(FIB_NUMBER);
	}

	fib(FIB_NUMBER)
}
