use core_affinity::CoreId;
use range_set_blaze::RangeSetBlaze;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug)]
pub struct CpuRange(RangeSetBlaze<usize>);
impl CpuRange {
    pub fn contains(&self, value: CoreId) -> bool {
        self.0.contains(value.id)
    }
}

impl TryFrom<RangeSetBlaze<usize>> for CpuRange {
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

        Ok(CpuRange(range))
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
impl FromStr for CpuRange {
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

        CpuRange::try_from(result).map_err(|err| ParseError::CpuRangeError { err })
    }
}

#[cfg(test)]
mod tests {
    use crate::cpu_range::{CpuRange, Error, ParseError};
    use core_affinity::CoreId;
    use range_set_blaze::RangeSetBlaze;

    #[test]
    fn range_parsing_test() {
        let total_cpus = num_cpus::get();
        let raw_str = format!("0-{}", total_cpus - 1);
        let cpu_range: CpuRange = raw_str.parse().unwrap();
        for value in 0..total_cpus {
            assert!(cpu_range.contains(CoreId { id: value }));
        }
    }

    #[test]
    fn values_parsing_test() {
        let total_cpus = num_cpus::get();
        let raw_str = format!("0,{}", total_cpus - 1);
        let cpu_range: CpuRange = raw_str.parse().unwrap();
        assert!(cpu_range.contains(CoreId { id: 0 }));
        if total_cpus > 1 {
            assert!(cpu_range.contains(CoreId { id: total_cpus - 1 }));
        }
    }

    #[test]
    fn wrong_parsing_test() {
        let result = "aaaa".parse::<CpuRange>();
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
        let result = "1-a".parse::<CpuRange>();
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
        let result = "a-1".parse::<CpuRange>();
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
        let result = "a-1-2,3".parse::<CpuRange>();
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
        let result = "".parse::<CpuRange>();
        assert!(result.is_err());
        if let Err(err) = result {
            assert_eq!(err, ParseError::EmptyRange);
        }
    }

    #[test]
    fn big_cpu_count() {
        let available_cores = num_cpus::get();
        let str = format!("0-{}", available_cores + 1);
        let result = str.parse::<CpuRange>();
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
        let cpu_range = CpuRange::try_from(range);
        assert!(cpu_range.is_err());
        if let Err(err) = cpu_range {
            assert_eq!(err, Error::EmptyRange)
        }
    }
}
