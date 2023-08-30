//! Compact histogram representation which is useful for serialization when the
//! data is sparse.

use crate::Config;
use crate::_Histograms;
use core::sync::atomic::Ordering;

/// A compact histogram which stores bucket indices and counts to efficiently
/// represent a sparse histogram.
#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Histogram {
    a: u8,
    b: u8,
    n: u8,
    pub(crate) index: Vec<usize>,
    pub(crate) count: Vec<u64>,
}

impl _Histograms for Histogram {
    fn config(&self) -> Config {
        Config::new(self.a, self.b, self.n).unwrap()
    }

    fn total_count(&self) -> u128 {
        self.count.iter().map(|c| *c as u128).sum()
    }

    fn get_count(&self, index: usize) -> u64 {
        if let Ok(index) = self.index.binary_search(&index) {
            *self.count.get(index).unwrap_or(&0)
        } else {
            0
        }
    }
}

impl From<crate::Histogram> for Histogram {
    fn from(other: crate::Histogram) -> Self {
        let (a, b, n) = other.config().params();
        let mut index = Vec::new();
        let mut count = Vec::new();

        for (i, c) in other.buckets.iter().enumerate().filter(|(_i, c)| **c != 0) {
            index.push(i);
            count.push(*c);
        }

        Self {
            a,
            b,
            n,
            index,
            count,
        }
    }
}

impl From<&crate::Histogram> for Histogram {
    fn from(other: &crate::Histogram) -> Self {
        let (a, b, n) = other.config().params();
        let mut index = Vec::new();
        let mut count = Vec::new();

        for (i, c) in other.buckets.iter().enumerate().filter(|(_i, c)| **c != 0) {
            index.push(i);
            count.push(*c);
        }

        Self {
            a,
            b,
            n,
            index,
            count,
        }
    }
}

impl From<crate::atomic::Histogram> for Histogram {
    fn from(other: crate::atomic::Histogram) -> Self {
        let (a, b, n) = other.config().params();
        let mut index = Vec::new();
        let mut count = Vec::new();

        for (i, c) in other
            .buckets
            .iter()
            .map(|c| c.load(Ordering::Relaxed))
            .enumerate()
            .filter(|(_i, c)| *c != 0)
        {
            index.push(i);
            count.push(c);
        }

        Self {
            a,
            b,
            n,
            index,
            count,
        }
    }
}

impl From<&crate::atomic::Histogram> for Histogram {
    fn from(other: &crate::atomic::Histogram) -> Self {
        let (a, b, n) = other.config().params();
        let mut index = Vec::new();
        let mut count = Vec::new();

        for (i, c) in other
            .buckets
            .iter()
            .map(|c| c.load(Ordering::Relaxed))
            .enumerate()
            .filter(|(_i, c)| *c != 0)
        {
            index.push(i);
            count.push(c);
        }

        Self {
            a,
            b,
            n,
            index,
            count,
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Histogram {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[allow(non_camel_case_types)]
        #[derive(serde::Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            A,
            B,
            N,
            Index,
            Count,
            ignore,
        }

        struct Visitor<'de> {
            marker: core::marker::PhantomData<Histogram>,
            lifetime: core::marker::PhantomData<&'de ()>,
        }

        impl<'de> serde::de::Visitor<'de> for Visitor<'de> {
            type Value = Histogram;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                std::fmt::Formatter::write_str(formatter, "struct Histogram")
            }
            #[inline]
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let a = match serde::de::SeqAccess::next_element::<u8>(&mut seq)? {
                    Some(value) => value,
                    None => {
                        return Err(serde::de::Error::invalid_length(
                            0usize,
                            &"struct Histogram with 5 elements",
                        ));
                    }
                };
                let b = match serde::de::SeqAccess::next_element::<u8>(&mut seq)? {
                    Some(value) => value,
                    None => {
                        return Err(serde::de::Error::invalid_length(
                            1usize,
                            &"struct Histogram with 5 elements",
                        ));
                    }
                };
                let n = match serde::de::SeqAccess::next_element::<u8>(&mut seq)? {
                    Some(value) => value,
                    None => {
                        return Err(serde::de::Error::invalid_length(
                            2usize,
                            &"struct Histogram with 5 elements",
                        ));
                    }
                };
                let index = match serde::de::SeqAccess::next_element::<Vec<usize>>(&mut seq)? {
                    Some(value) => value,
                    None => {
                        return Err(serde::de::Error::invalid_length(
                            3usize,
                            &"struct Histogram with 5 elements",
                        ));
                    }
                };
                let count = match serde::de::SeqAccess::next_element::<Vec<u64>>(&mut seq)? {
                    Some(value) => value,
                    None => {
                        return Err(serde::de::Error::invalid_length(
                            4usize,
                            &"struct Histogram with 5 elements",
                        ));
                    }
                };
                Ok(Histogram {
                    a,
                    b,
                    n,
                    index,
                    count,
                })
            }

            #[inline]
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut a: Option<u8> = None;
                let mut b: Option<u8> = None;
                let mut n: Option<u8> = None;
                let mut index: Option<Vec<usize>> = None;
                let mut count: Option<Vec<u64>> = None;
                while let Some(key) = serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::A => {
                            if a.is_some() {
                                return Err(<A::Error as serde::de::Error>::duplicate_field("a"));
                            }
                            a = Some(serde::de::MapAccess::next_value::<u8>(&mut map)?);
                        }
                        Field::B => {
                            if b.is_some() {
                                return Err(<A::Error as serde::de::Error>::duplicate_field("b"));
                            }
                            b = Some(serde::de::MapAccess::next_value::<u8>(&mut map)?);
                        }
                        Field::N => {
                            if n.is_some() {
                                return Err(<A::Error as serde::de::Error>::duplicate_field("n"));
                            }
                            n = Some(serde::de::MapAccess::next_value::<u8>(&mut map)?);
                        }
                        Field::Index => {
                            if index.is_some() {
                                return Err(<A::Error as serde::de::Error>::duplicate_field(
                                    "index",
                                ));
                            }
                            index = Some(serde::de::MapAccess::next_value::<Vec<usize>>(&mut map)?);
                        }
                        Field::Count => {
                            if count.is_some() {
                                return Err(<A::Error as serde::de::Error>::duplicate_field(
                                    "count",
                                ));
                            }
                            count = Some(serde::de::MapAccess::next_value::<Vec<u64>>(&mut map)?);
                        }
                        _ => {
                            let _ = serde::de::MapAccess::next_value::<serde::de::IgnoredAny>(
                                &mut map,
                            )?;
                        }
                    }
                }

                let a = a.ok_or_else(|| serde::de::Error::missing_field("a"))?;
                let b = b.ok_or_else(|| serde::de::Error::missing_field("b"))?;
                let n = n.ok_or_else(|| serde::de::Error::missing_field("n"))?;
                let index = index.ok_or_else(|| serde::de::Error::missing_field("index"))?;
                let count = count.ok_or_else(|| serde::de::Error::missing_field("count"))?;

                if n > 64 {
                    return Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Unsigned(n.into()),
                        &"n must be in the range 0..=64",
                    ));
                }

                if n < a + b {
                    return Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Unsigned(n.into()),
                        &"n must be greater than a + b",
                    ));
                }

                if index.len() != count.len() {
                    return Err(serde::de::Error::custom(
                        "index and count vectors have mismatched lengths",
                    ));
                }

                Ok(Histogram {
                    a,
                    b,
                    n,
                    index,
                    count,
                })
            }
        }

        const FIELDS: &[&str] = &["a", "b", "n", "index", "count"];

        serde::Deserializer::deserialize_struct(
            deserializer,
            "Histogram",
            FIELDS,
            Visitor {
                marker: core::marker::PhantomData::<Histogram>,
                lifetime: core::marker::PhantomData,
            },
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[cfg(feature = "serde")]
    #[test]
    fn serde_ser() {
        let mut h = crate::Histogram::new(0, 7, 64).unwrap();

        let c: Histogram = (&h).into();

        assert_eq!(
            serde_json::to_string(&c).unwrap(),
            "{\"a\":0,\"b\":7,\"n\":64,\"index\":[],\"count\":[]}"
        );

        let _ = h.increment(0);

        let c: Histogram = (&h).into();

        assert_eq!(
            serde_json::to_string(&c).unwrap(),
            "{\"a\":0,\"b\":7,\"n\":64,\"index\":[0],\"count\":[1]}"
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_de() {
        assert!(serde_json::from_str::<Histogram>(
            "{\"a\":0,\"b\":7,\"n\":64,\"index\":[],\"count\":[]}"
        )
        .is_ok());

        assert!(serde_json::from_str::<Histogram>(
            "{\"a\":0,\"b\":7,\"n\":0,\"index\":[],\"count\":[]}"
        )
        .is_err());
        assert!(serde_json::from_str::<Histogram>(
            "{\"a\":0,\"b\":7,\"n\":65,\"index\":[],\"count\":[]}"
        )
        .is_err());
    }
}
