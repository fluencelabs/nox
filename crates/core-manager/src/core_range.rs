use range_set_blaze::RangeSetBlaze;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct CoreRange(pub(crate) RangeSetBlaze<usize>);

impl CoreRange {
    pub fn last(&self) -> usize {
        self.0.last().expect("Non empty list")
    }
}

impl TryFrom<&[usize]> for CoreRange {
    type Error = ParseError;

    fn try_from(value: &[usize]) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(ParseError::EmptyRange);
        }
        Ok(CoreRange(RangeSetBlaze::from_iter(value)))
    }
}

impl FromStr for CoreRange {
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

        Ok(CoreRange(result))
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum ParseError {
    #[error("Range can't be an empty")]
    EmptyRange,
    #[error("Failed to parse str: {raw_str}")]
    WrongRangeFormat { raw_str: String },
}

#[cfg(test)]
mod tests {
    use crate::core_range::{CoreRange, ParseError};

    #[test]
    fn range_parsing_test() {
        let core_range: CoreRange = "0-2".parse().unwrap();
        assert!(core_range.0.contains(0));
        assert!(core_range.0.contains(1));
        assert!(core_range.0.contains(2));
        assert!(!core_range.0.contains(3));
    }

    #[test]
    fn values_parsing_test() {
        let core_range: CoreRange = "0,1,3".parse().unwrap();
        assert!(core_range.0.contains(0));
        assert!(core_range.0.contains(1));
        assert!(!core_range.0.contains(2));
        assert!(core_range.0.contains(3));
    }

    #[test]
    fn wrong_parsing_test() {
        let result = "aaaa".parse::<CoreRange>();
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
        let result = "1-a".parse::<CoreRange>();
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
        let result = "a-1".parse::<CoreRange>();
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
        let result = "a-1-2,3".parse::<CoreRange>();
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
        let result = "".parse::<CoreRange>();
        assert!(result.is_err());
        if let Err(err) = result {
            assert_eq!(err, ParseError::EmptyRange);
        }
    }
}
