use crate::core_range::CoreRange;
use range_set_blaze::RangeSetBlaze;
use thiserror::Error;

#[derive(Debug)]
pub struct CoreSet(pub(crate) RangeSetBlaze<usize>);

impl TryFrom<CoreRange> for CoreSet {
    type Error = Error;

    fn try_from(range: CoreRange) -> Result<Self, Self::Error> {
        let available_cores = num_cpus::get();
        let range_max = range.last();
        if range_max > available_cores {
            return Err(Error::RangeIsTooBig {
                available_cores,
                range,
            });
        }

        Ok(CoreSet(range.0))
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum Error {
    #[error("Range can't be an empty")]
    EmptyRange,
    #[error("Range is too big. Available cores: {available_cores}, requested range: {range:?}")]
    RangeIsTooBig {
        available_cores: usize,
        range: CoreRange,
    },
}

#[cfg(test)]
mod tests {
    use crate::core_range::CoreRange;
    use crate::core_set::{CoreSet, Error};

    #[test]
    fn big_cpu_count() {
        let available_cores = num_cpus::get();
        let str = format!("0-{}", available_cores + 1);
        let range = str.parse::<CoreRange>().unwrap();
        let core_set = CoreSet::try_from(range.clone());
        assert!(core_set.is_err());
        if let Err(err) = core_set {
            assert_eq!(
                err,
                Error::RangeIsTooBig {
                    available_cores,
                    range
                }
            );
        }
    }
}
