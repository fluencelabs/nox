use ccp_shared::types::PhysicalCoreId;
use nonempty::NonEmpty;
use range_set_blaze::RangeSetBlaze;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

#[derive(Clone, PartialEq)]
pub struct CoreRange(pub(crate) RangeSetBlaze<usize>);

impl CoreRange {
    pub fn is_subset(&self, cores: &NonEmpty<PhysicalCoreId>) -> bool {
        let range: RangeSetBlaze<usize> =
            RangeSetBlaze::from_iter(cores.into_iter().map(|core| <usize>::from(*core)));

        self.0.is_subset(&range)
    }
}

impl Debug for CoreRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.to_string().as_str())
    }
}

impl Default for CoreRange {
    fn default() -> Self {
        CoreRange(RangeSetBlaze::from_iter(0..num_cpus::get_physical()))
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

    /// Parse CoreRange from string like "1,2-30,31"
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut result: RangeSetBlaze<usize> = RangeSetBlaze::new();
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(ParseError::EmptyRange);
        }

        for part in trimmed.split(',') {
            let trimmed = part.trim();
            // either a single number or a dash range
            let range: Vec<&str> = trimmed.split('-').collect();
            match range[..] {
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
                    // insert the inclusive range
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
                    });
                }
            }
        }

        Ok(CoreRange(result))
    }
}

impl Display for CoreRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (index, range) in self.0.ranges().enumerate() {
            if index != 0 {
                write!(f, ",")?;
            };
            let start = range.start();
            let end = range.end();
            if start == end {
                write!(f, "{}", start)?;
            } else {
                write!(f, "{}-{}", start, end)?;
            }
        }
        Ok(())
    }
}

impl Serialize for CoreRange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl<'de> Deserialize<'de> for CoreRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_str = String::deserialize(deserializer)?;
        CoreRange::from_str(raw_str.as_str()).map_err(|e| {
            serde::de::Error::custom(format!("failed to deserialize core range {raw_str} {e:?}"))
        })
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum ParseError {
    #[error("Range can't be an empty")]
    EmptyRange,
    #[error("Failed to parse: {raw_str}")]
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
            assert_eq!(err.to_string(), "Failed to parse: aaaa")
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
            assert_eq!(err.to_string(), "Failed to parse: 1-a")
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
            assert_eq!(err.to_string(), "Failed to parse: a-1")
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
            assert_eq!(err.to_string(), "Failed to parse: a-1-2")
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

    #[test]
    fn slice_convert() {
        let core_range = CoreRange::try_from(&vec![1, 2, 3, 10][..]).unwrap();

        assert!(!core_range.0.contains(0));
        assert!(core_range.0.contains(1));
        assert!(core_range.0.contains(2));
        assert!(core_range.0.contains(3));
        assert!(core_range.0.contains(10));
        assert!(!core_range.0.contains(11));
    }

    #[test]
    fn empty_slice_convert() {
        let result = CoreRange::try_from(&vec![][..]);

        assert!(result.is_err());
        if let Err(err) = result {
            assert_eq!(err, ParseError::EmptyRange);
            assert_eq!(err.to_string(), "Range can't be an empty")
        }
    }

    #[test]
    fn last() {
        let core_range: CoreRange = "0-2".parse().unwrap();
        assert!(core_range.0.contains(0));
        assert!(core_range.0.contains(1));
        assert!(core_range.0.contains(2));
        assert!(!core_range.0.contains(3));
    }

    #[test]
    fn compare_ranges() {
        let core_range_1: CoreRange = "0-2".parse().unwrap();
        let core_range_2: CoreRange = "0,1,2".parse().unwrap();
        assert_eq!(core_range_1, core_range_2);
    }

    #[test]
    fn fmt() {
        let core_range_1: CoreRange = "0-2,5,7-9".parse().unwrap();
        assert_eq!(format!("{}", core_range_1), "0-2,5,7-9");
    }

    #[test]
    fn range_is_inclusive() {
        let core_range_1: CoreRange = "1-3".parse().unwrap();
        let actual: Vec<usize> = core_range_1.0.iter().collect();
        let expected = vec![1, 2, 3];
        assert_eq!(actual, expected)
    }
}
