#[cfg(features = "std")]
use std::collections::{HashMap as Map};
#[cfg(not(features = "std"))]
use std::collections::{BTreeMap as Map};

use pwasm_utils::rules::{InstructionType, Metering};
use std::cell::RefCell;

//thread_local! {
    static COST_TABLE: RefCell<Map<InstructionType, Metering>> = RefCell::new(vec![
        (InstructionType::Add, Metering::Fixed(3)),
        (InstructionType::Bit, Metering::Fixed(3)),
        (InstructionType::Const, Metering::Fixed(3)),
        (InstructionType::ControlFlow, Metering::Fixed(3)),
        (InstructionType::Conversion, Metering::Fixed(3)),
        (InstructionType::CurrentMemory, Metering::Fixed(3)),
        (InstructionType::Div, Metering::Fixed(3)),
        (InstructionType::Float, Metering::Fixed(3)),
        (InstructionType::FloatComparsion, Metering::Fixed(3)),
        (InstructionType::FloatConst, Metering::Fixed(3)),
        (InstructionType::FloatConversion, Metering::Fixed(3)),
        (InstructionType::Global, Metering::Fixed(3)),
        (InstructionType::GrowMemory, Metering::Fixed(0)),
        (InstructionType::IntegerComparsion, Metering::Fixed(3)),
        (InstructionType::Load, Metering::Fixed(3)),
        (InstructionType::Local, Metering::Fixed(3)),
        (InstructionType::Mul, Metering::Fixed(3)),
        (InstructionType::Nop, Metering::Fixed(3)),
        (InstructionType::Reinterpretation, Metering::Fixed(3)),
        (InstructionType::Store, Metering::Fixed(3)),
        (InstructionType::Unreachable, Metering::Fixed(3))
    ].into_iter().collect()
);
//}
