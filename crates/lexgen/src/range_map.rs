use std::cmp::{max, min};

/// A map of inclusive ranges, with insertion and iteration operations. Insertion allows
/// overlapping ranges. When two ranges overlap, value of the overlapping parts is the union of
/// values of the overlapping ranges.
#[derive(Debug)]
pub struct RangeMap<A> {
    // NB. internally we don't have any overlaps. Overlapping ranges are split into smaller
    // non-overlapping ranges.
    ranges: Vec<Range<A>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Range<A> {
    pub start: u32,
    // Inclusive
    pub end: u32,
    pub value: A,
}

impl<A> Default for RangeMap<A> {
    fn default() -> Self {
        RangeMap::new()
    }
}

impl<A> RangeMap<A> {
    fn new() -> RangeMap<A> {
        RangeMap { ranges: vec![] }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Range<A>> {
        self.ranges.iter()
    }

    pub fn into_iter(self) -> impl Iterator<Item = Range<A>> {
        self.ranges.into_iter()
    }

    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    pub fn filter_map<F, B>(self, mut f: F) -> RangeMap<B>
    where
        F: FnMut(A) -> Option<B>,
    {
        RangeMap {
            ranges: self
                .ranges
                .into_iter()
                .filter_map(|Range { start, end, value }| {
                    f(value).map(|value| Range { start, end, value })
                })
                .collect(),
        }
    }
}

impl<A> Range<A> {
    pub fn contains(&self, char: char) -> bool {
        char as u32 >= self.start && char as u32 <= self.end
    }
}

impl<A: Clone> RangeMap<A> {
    /// O(n) where n is the number of existing ranges in the map
    pub fn insert<F>(&mut self, mut new_range_start: u32, new_range_end: u32, value: A, merge: F)
    where
        F: Fn(&mut A, A),
    {
        let old_ranges = std::mem::replace(&mut self.ranges, vec![]);
        let mut new_ranges = Vec::with_capacity(old_ranges.len() + 2);

        let mut range_iter = old_ranges.into_iter();

        while let Some(range) = range_iter.next() {
            if range.end < new_range_start {
                new_ranges.push(range);
            } else if range.start > new_range_end {
                new_ranges.push(Range {
                    start: new_range_start,
                    end: new_range_end,
                    value,
                });
                new_ranges.push(range);
                new_ranges.extend(range_iter);
                self.ranges = new_ranges;
                return;
            } else {
                let overlap = max(new_range_start, range.start)..=min(new_range_end, range.end);

                // (1) push new_range before the overlap
                // (2) push old_range before the overlap
                // (3) push overlapping part
                // (4) push old_range after the overlap
                // (5) push new_range after the overlap
                //
                //
                // 1 and 2, 4 and 5 can't happen at once. 5 needs to be handled in the next
                // iteration as there may be other overlapping ranges with new_range after the
                // current overlap. In all other cases, we copy rest of the ranges and return.

                // (1)
                if new_range_start < *overlap.start() {
                    new_ranges.push(Range {
                        start: new_range_start,
                        end: *overlap.start() - 1,
                        value: value.clone(),
                    });
                }
                // (2)
                else if range.start < *overlap.start() {
                    new_ranges.push(Range {
                        start: range.start,
                        end: overlap.start() - 1,
                        value: range.value.clone(),
                    });
                }

                // (3)
                let mut overlap_values = range.value.clone();
                merge(&mut overlap_values, value.clone());
                new_ranges.push(Range {
                    start: *overlap.start(),
                    end: *overlap.end(),
                    value: overlap_values,
                });

                // (4)
                if range.end > *overlap.end() {
                    new_ranges.push(Range {
                        start: *overlap.end() + 1,
                        end: range.end,
                        value: range.value,
                    });
                }
                // (5)
                else if new_range_end > *overlap.end() {
                    new_range_start = *overlap.end() + 1;
                    continue;
                }

                new_ranges.extend(range_iter);
                self.ranges = new_ranges;
                return;
            }
        }

        let push_new_range = match new_ranges.last() {
            None => true,
            Some(last_range) => last_range.end < new_range_start,
        };

        if push_new_range {
            new_ranges.push(Range {
                start: new_range_start,
                end: new_range_end,
                value,
            });
        }

        self.ranges = new_ranges;
    }
}

#[cfg(test)]
fn to_tuple<A: Clone>(range: &Range<Vec<A>>) -> (u32, u32, Vec<A>) {
    (range.start, range.end, range.value.clone())
}

#[cfg(test)]
fn to_vec<A: Clone>(map: &RangeMap<Vec<A>>) -> Vec<(u32, u32, Vec<A>)> {
    map.iter().map(to_tuple).collect()
}

#[cfg(test)]
fn insert<A: Clone>(map: &mut RangeMap<Vec<A>>, range_start: u32, range_end: u32, value: A) {
    map.insert(range_start, range_end, vec![value], |values_1, values_2| {
        values_1.extend(values_2.into_iter())
    });
}

#[test]
fn overlap_left() {
    let mut ranges: RangeMap<Vec<u32>> = RangeMap::new();

    insert(&mut ranges, 10, 20, 0);
    insert(&mut ranges, 5, 15, 1);

    assert_eq!(
        to_vec(&ranges),
        vec![(5, 9, vec![1]), (10, 15, vec![0, 1]), (16, 20, vec![0])]
    );

    insert(&mut ranges, 5, 5, 2);

    assert_eq!(
        to_vec(&ranges),
        vec![
            (5, 5, vec![1, 2]),
            (6, 9, vec![1]),
            (10, 15, vec![0, 1]),
            (16, 20, vec![0]),
        ]
    );

    let mut ranges: RangeMap<Vec<u32>> = RangeMap::new();

    insert(&mut ranges, 10, 20, 0);
    insert(&mut ranges, 10, 15, 1);

    assert_eq!(
        to_vec(&ranges),
        vec![(10, 15, vec![0, 1]), (16, 20, vec![0])]
    );
}

#[test]
fn overlap_right() {
    let mut ranges: RangeMap<Vec<u32>> = RangeMap::new();

    insert(&mut ranges, 5, 15, 1);

    assert_eq!(to_vec(&ranges), vec![(5, 15, vec![1])]);

    insert(&mut ranges, 10, 20, 0);

    assert_eq!(
        to_vec(&ranges),
        vec![(5, 9, vec![1]), (10, 15, vec![1, 0]), (16, 20, vec![0])]
    );

    insert(&mut ranges, 20, 20, 2);

    assert_eq!(
        to_vec(&ranges),
        vec![
            (5, 9, vec![1]),
            (10, 15, vec![1, 0]),
            (16, 19, vec![0]),
            (20, 20, vec![0, 2]),
        ]
    );

    let mut ranges: RangeMap<Vec<u32>> = RangeMap::new();

    insert(&mut ranges, 10, 15, 1);
    insert(&mut ranges, 10, 20, 0);

    assert_eq!(
        to_vec(&ranges),
        vec![(10, 15, vec![1, 0]), (16, 20, vec![0])]
    );
}

#[test]
fn add_non_overlapping() {
    let mut ranges: RangeMap<Vec<u32>> = RangeMap::new();

    insert(&mut ranges, 0, 10, 1);
    insert(&mut ranges, 20, 30, 0);

    assert_eq!(to_vec(&ranges), vec![(0, 10, vec![1]), (20, 30, vec![0]),]);
}

#[test]
fn add_non_overlapping_reverse() {
    let mut ranges: RangeMap<Vec<u32>> = RangeMap::new();

    insert(&mut ranges, 20, 30, 0);
    insert(&mut ranges, 0, 10, 1);

    assert_eq!(to_vec(&ranges), vec![(0, 10, vec![1]), (20, 30, vec![0]),]);
}

#[test]
fn add_overlapping_1() {
    let mut ranges: RangeMap<Vec<u32>> = RangeMap::new();

    insert(&mut ranges, 0, 10, 0);
    insert(&mut ranges, 10, 20, 1);

    assert_eq!(
        to_vec(&ranges),
        vec![(0, 9, vec![0]), (10, 10, vec![0, 1]), (11, 20, vec![1]),]
    );
}

#[test]
fn add_overlapping_1_reverse() {
    let mut ranges: RangeMap<Vec<u32>> = RangeMap::new();

    insert(&mut ranges, 10, 20, 1);
    insert(&mut ranges, 0, 10, 0);

    assert_eq!(
        to_vec(&ranges),
        vec![(0, 9, vec![0]), (10, 10, vec![1, 0]), (11, 20, vec![1]),]
    );
}

#[test]
fn add_overlapping_2() {
    let mut ranges: RangeMap<Vec<u32>> = RangeMap::new();

    insert(&mut ranges, 50, 100, 0);

    assert_eq!(to_vec(&ranges), vec![(50, 100, vec![0])]);

    insert(&mut ranges, 40, 60, 1);

    assert_eq!(
        to_vec(&ranges),
        vec![(40, 49, vec![1]), (50, 60, vec![0, 1]), (61, 100, vec![0]),]
    );

    insert(&mut ranges, 90, 110, 2);

    assert_eq!(
        to_vec(&ranges),
        vec![
            (40, 49, vec![1]),
            (50, 60, vec![0, 1]),
            (61, 89, vec![0]),
            (90, 100, vec![0, 2]),
            (101, 110, vec![2]),
        ]
    );

    insert(&mut ranges, 70, 80, 3);

    assert_eq!(
        to_vec(&ranges),
        vec![
            (40, 49, vec![1]),
            (50, 60, vec![0, 1]),
            (61, 69, vec![0]),
            (70, 80, vec![0, 3]),
            (81, 89, vec![0]),
            (90, 100, vec![0, 2]),
            (101, 110, vec![2]),
        ]
    );
}

#[test]
fn large_range_multiple_overlaps() {
    let mut ranges: RangeMap<Vec<u32>> = RangeMap::new();

    insert(&mut ranges, 10, 20, 0);
    insert(&mut ranges, 21, 30, 1);
    insert(&mut ranges, 5, 35, 2);

    assert_eq!(
        to_vec(&ranges),
        vec![
            (5, 9, vec![2]),
            (10, 20, vec![0, 2]),
            (21, 30, vec![1, 2]),
            (31, 35, vec![2]),
        ]
    );
}

#[test]
fn overlap_middle() {
    let mut ranges: RangeMap<Vec<u32>> = RangeMap::new();

    insert(&mut ranges, 10, 20, 0);
    insert(&mut ranges, 15, 15, 1);

    assert_eq!(
        to_vec(&ranges),
        vec![(10, 14, vec![0]), (15, 15, vec![0, 1]), (16, 20, vec![0])]
    );
}

#[test]
fn overlap_exact() {
    let mut ranges: RangeMap<Vec<u32>> = RangeMap::new();

    insert(&mut ranges, 10, 20, 0);
    insert(&mut ranges, 10, 20, 1);

    assert_eq!(to_vec(&ranges), vec![(10, 20, vec![0, 1])]);
}
