use core_affinity::CoreId;
use range_set_blaze::RangeSetBlaze;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug)]
pub struct CoreSet(pub(crate) RangeSetBlaze<usize>);
impl CoreSet {
    pub fn to_vec(&self) -> Vec<CoreId> {
        self.0.iter().map(|id| CoreId { id }).collect()
    }
}

impl TryFrom<RangeSetBlaze<usize>> for CoreSet {
    type Error = Error;

    fn try_from(range: RangeSetBlaze<usize>) -> Result<Self, Self::Error> {
        if range.is_empty() {
            return Err(Error::EmptyRange);
        }
        let available_cores = num_cpus::get();
        let range_max = range.last().unwrap(); //last always exists, can't be empty
        if range_max > available_cores {
            return Err(Error::RangeIsTooBig { available_cores });
        }

        Ok(CoreSet(range))
    }
}

impl TryFrom<&[usize]> for CoreSet {
    type Error = Error;

    fn try_from(values: &[usize]) -> Result<Self, Self::Error> {
        let set = RangeSetBlaze::from_iter(values.iter());
        CoreSet::try_from(set)
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum Error {
    #[error("Range can't be an empty")]
    EmptyRange,
    #[error("Range is too big. Available cores: {available_cores}")]
    RangeIsTooBig { available_cores: usize },
}

#[derive(Debug, Error, PartialEq)]
pub enum ParseError {
    #[error("Range can't be an empty")]
    EmptyRange,
    #[error("Failed to build a cpu range: {err}")]
    CpuRangeError {
        #[source]
        err: Error,
    },
    #[error("Failed to parse str: {raw_str}")]
    WrongRangeFormat { raw_str: String },
}
impl FromStr for CoreSet {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut result: RangeSetBlaze<usize> = RangeSetBlaze::new();
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(ParseError::EmptyRange);
        }

        for part in trimmed.split(',') {
            let trimmed = part.trim();
            let split: Vec<&str> = trimmed.split('-').collect();
            match split[..] {
                [l, r] => {
                    let l = l
                        .parse::<usize>()
                        .map_err(|_| ParseError::WrongRangeFormat {
                            raw_str: trimmed.to_string(),
                        })?;
                    let r = r
                        .parse::<usize>()
                        .map_err(|_| ParseError::WrongRangeFormat {
                            raw_str: trimmed.to_string(),
                        })?;
                    result.ranges_insert(l..=r);
                }
                [value] => {
                    let value =
                        value
                            .parse::<usize>()
                            .map_err(|_| ParseError::WrongRangeFormat {
                                raw_str: trimmed.to_string(),
                            })?;
                    result.insert(value);
                }
                _ => {
                    return Err(ParseError::WrongRangeFormat {
                        raw_str: trimmed.to_string(),
                    })
                }
            }
        }

        CoreSet::try_from(result).map_err(|err| ParseError::CpuRangeError { err })
    }
}

#[cfg(test)]
mod tests {
    use crate::core_set::{CoreSet, Error, ParseError};
    use range_set_blaze::RangeSetBlaze;

    #[test]
    fn range_parsing_test() {
        let total_cpus = num_cpus::get();
        let raw_str = format!("0-{}", total_cpus - 1);
        let cpu_range: CoreSet = raw_str.parse().unwrap();
        for value in 0..total_cpus {
            assert!(cpu_range.0.contains(value));
        }
    }

    #[test]
    fn values_parsing_test() {
        let total_cpus = num_cpus::get();
        let raw_str = format!("0,{}", total_cpus - 1);
        let cpu_range: CoreSet = raw_str.parse().unwrap();
        assert!(cpu_range.0.contains(0));
        if total_cpus > 1 {
            assert!(cpu_range.0.contains(total_cpus - 1));
        }
    }

    #[test]
    fn wrong_parsing_test() {
        let result = "aaaa".parse::<CoreSet>();
        assert!(result.is_err());
        if let Err(err) = result {
            assert_eq!(
                err,
                ParseError::WrongRangeFormat {
                    raw_str: "aaaa".to_string()
                }
            );
        }
    }
    #[test]
    fn wrong_parsing_test_2() {
        let result = "1-a".parse::<CoreSet>();
        assert!(result.is_err());
        if let Err(err) = result {
            assert_eq!(
                err,
                ParseError::WrongRangeFormat {
                    raw_str: "1-a".to_string()
                }
            );
        }
    }
    #[test]
    fn wrong_parsing_test_3() {
        let result = "a-1".parse::<CoreSet>();
        assert!(result.is_err());
        if let Err(err) = result {
            assert_eq!(
                err,
                ParseError::WrongRangeFormat {
                    raw_str: "a-1".to_string()
                }
            );
        }
    }

    #[test]
    fn wrong_parsing_test_4() {
        let result = "a-1-2,3".parse::<CoreSet>();
        assert!(result.is_err());
        if let Err(err) = result {
            assert_eq!(
                err,
                ParseError::WrongRangeFormat {
                    raw_str: "a-1-2".to_string()
                }
            );
        }
    }

    #[test]
    fn empty_parsing_test_3() {
        let result = "".parse::<CoreSet>();
        assert!(result.is_err());
        if let Err(err) = result {
            assert_eq!(err, ParseError::EmptyRange);
        }
    }

    #[test]
    fn big_cpu_count() {
        let available_cores = num_cpus::get();
        let str = format!("0-{}", available_cores + 1);
        let result = str.parse::<CoreSet>();
        assert!(result.is_err());
        if let Err(err) = result {
            assert_eq!(
                err,
                ParseError::CpuRangeError {
                    err: Error::RangeIsTooBig { available_cores }
                }
            );
        }
    }

    #[test]
    fn empty_range_check() {
        let range: RangeSetBlaze<usize> = RangeSetBlaze::new();
        let cpu_range = CoreSet::try_from(range);
        assert!(cpu_range.is_err());
        if let Err(err) = cpu_range {
            assert_eq!(err, Error::EmptyRange)
        }
    }
}
