//! CarriedUsage.swift: a reader upgrade replaces earlier accounting by covered
//! source time, then drops the remaining carry only after it has read the file.
use serde::{Deserialize, Serialize};
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct CarriedUsage {
    span: Option<(i64, i64)>,
}
impl CarriedUsage {
    pub fn writing(
        &self,
        times: impl Iterator<Item = i64>,
        whole: bool,
    ) -> (Vec<(i64, i64)>, Option<Self>) {
        if whole {
            return (
                match self.span {
                    Some((first, last)) => {
                        let mut ranges = Vec::new();
                        if first > i64::MIN {
                            ranges.push((i64::MIN, first - 1));
                        }
                        if last < i64::MAX {
                            ranges.push((last + 1, i64::MAX));
                        }
                        ranges
                    }
                    None => vec![(i64::MIN, i64::MAX)],
                },
                None,
            );
        }
        let mut times = times;
        let Some(first) = times.next() else {
            return (Vec::new(), Some(self.clone()));
        };
        let (first, last) = times.fold((first, first), |(min, max), at| (min.min(at), max.max(at)));
        match self.span {
            None => (
                vec![(first, last)],
                Some(Self {
                    span: Some((first, last)),
                }),
            ),
            Some((old_first, old_last)) => {
                let mut ranges = Vec::new();
                if first < old_first {
                    ranges.push((first, old_first - 1));
                }
                if last > old_last {
                    ranges.push((old_last + 1, last));
                }
                (
                    ranges,
                    Some(Self {
                        span: Some((first.min(old_first), last.max(old_last))),
                    }),
                )
            }
        }
    }
}
