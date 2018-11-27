// this seed is used for deterministic operation count on different launches
pub const SEED : &'static str = env!("SEED");

// matrix size
pub const MATRIX_SIZE : &'static str = env!("MATRIX_SIZE");

// count of test iterations
pub const ITERATIONS_COUNT : &'static str  = env!("ITERATIONS_COUNT");

// 1117 due to prevent overflow in matrix multiplication
pub const GENERATION_INTERVAL : f64 = 1117.0;

pub extern crate nalgebra;
use nalgebra::DMatrix;
// exactly matrix type
pub type Matrix = DMatrix<f64>;
