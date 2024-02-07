use crate::core_range::CoreRange;
use range_set_blaze::RangeSetBlaze;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct CoreSet(pub(crate) RangeSetBlaze<usize>);

impl CoreSet {
    pub fn insert(&mut self, value: usize) -> bool {
        self.0.insert(value)
    }
}

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
    #[error("Range is too big. Available cores: {available_cores}, requested range: {range:?}")]
    RangeIsTooBig {
        available_cores: usize,
        range: CoreRange,
    },
}

impl Default for CoreSet {
    fn default() -> Self {
        let cpu_count = num_cpus::get();
        CoreSet(RangeSetBlaze::from_iter(0..cpu_count))
    }
}

#[cfg(test)]
mod tests {
    use crate::core_range::CoreRange;
    use crate::core_set::{CoreSet, Error};

    #[test]
    fn simple_set_test() {
        let available_cores = num_cpus::get();
        let str = format!("0-{}", available_cores - 1);
        let range = str.parse::<CoreRange>().unwrap();
        let core_set = CoreSet::try_from(range.clone());
        assert!(core_set.is_ok());
        if let Ok(core_set) = core_set {
            assert_eq!(
                format!("{:?}", core_set),
                format!("CoreSet(0..={})", available_cores - 1)
            )
        }
    }

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
            assert_eq!(
                err.to_string(),
                "Range is too big. Available cores: 8, requested range: 0..=9"
            );
        }
    }
}
