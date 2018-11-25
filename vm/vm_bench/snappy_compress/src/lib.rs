extern crate snap;
use settings;

type Sequence = Vec<u8>;

#[no_mangle]
pub extern "C" fn generate_sequence(size : i64) -> Sequence {

}

#[no_mangle]
pub extern "C" fn benchmark_entry() -> i64 {
    const SIZE: usize = 1048576 * 8;
    let bytes: Vec<u8> = vec! [ 0; SIZE ];
    let encoded = snap::Encoder::new().compress_vec(&bytes).unwrap();

    for _ in 1..ITERATIONS_COUNT {
        let sequence = generate_sequence()
    }

    encoded.len() as i64
}
