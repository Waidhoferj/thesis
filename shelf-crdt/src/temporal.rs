use crate::{Incrementable, Mergeable};
use bloom::{BloomFilter, Intersectable};
use serde::{Deserialize, Serialize};
use std::{borrow::BorrowMut, cmp::Ordering, collections::HashMap};

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Temporal {
    // Lamport timestamp takes client id. This would eliminate vector clock
    LamportTS(u32),
    // Eliminate this
    VectorClock {
        clocks: HashMap<String, u32>,
        user_id: String,
    },
}

impl Incrementable for Temporal {
    fn increment(&mut self) {
        match self {
            Temporal::LamportTS(i) => *i += 1,
            Temporal::VectorClock { clocks, user_id } => *clocks.get_mut(user_id).unwrap() += 1,
        }
    }
}

impl Default for Temporal {
    fn default() -> Self {
        Temporal::LamportTS(0)
    }
}

impl PartialOrd for Temporal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Temporal::LamportTS(this), Temporal::LamportTS(other)) => this.partial_cmp(other),
            (
                Temporal::VectorClock { clocks: this, .. },
                Temporal::VectorClock { clocks: other, .. },
            ) => {
                let mut seen_greater = false;
                let mut seen_less = false;
                let mut intersected = 0;
                for (user, this_clock) in this.iter() {
                    if let Some(other_clock) = other.get(user) {
                        intersected += 1;
                        let comp = this_clock.cmp(other_clock);
                        match comp {
                            Ordering::Greater => seen_greater = true,
                            Ordering::Less => seen_less = true,
                            Ordering::Equal => (),
                        }
                        if seen_greater && seen_less {
                            return None;
                        }
                    }
                }
                if seen_greater {
                    Some(Ordering::Greater)
                } else if seen_less {
                    Some(Ordering::Less)
                } else if intersected == 0 {
                    None
                } else {
                    Some(Ordering::Equal)
                }
            }
            (Temporal::LamportTS(_), Temporal::VectorClock { .. }) => Some(Ordering::Less),
            (Temporal::VectorClock { .. }, Temporal::LamportTS(_)) => Some(Ordering::Greater),
        }
    }
}

impl Mergeable for Temporal {
    type Other = Self;
    fn merge(mut self, other: Self) -> Self {
        match (self.borrow_mut(), other) {
            (Temporal::LamportTS(first), Temporal::LamportTS(second)) => {
                Temporal::LamportTS((*first).max(second))
            }
            (
                Temporal::VectorClock {
                    clocks: first,
                    user_id,
                },
                Temporal::VectorClock { clocks: second, .. },
            ) => {
                for (key, val) in second {
                    first
                        .entry(key)
                        .and_modify(|v| *v = (*v).max(val))
                        .or_insert(val);
                }
                Temporal::VectorClock {
                    clocks: first.to_owned(),
                    user_id: user_id.to_owned(),
                }
            }
            _ => panic!("Temporal: Compared unmatched types."),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::RandomState;

    use bloom::ASMS;

    use super::*;

    #[test]
    fn test_intersection() {
        let h1 = RandomState::new();
        let h2 = RandomState::new();
        let mut b1 = BloomFilter::with_rate_and_hashers(0.01, 1000, h1.clone(), h2.clone());
        let mut b2 = BloomFilter::with_rate_and_hashers(0.01, 1000, h1, h2);
        // Empty sets don't intersect
        assert!(!b1.intersect(&b2));
        assert!(!b2.intersect(&b1));

        b1.insert(&(1, 2));
        b1.insert(&(1, 3));
        b2.insert(&(1, 2));
        // Intersects is basically a superset operation
        assert!(b1.intersect(&b2));
        assert!(!b2.intersect(&b1));

        b2.insert(&(1, 4));
        assert!(!b1.intersect(&b2));
        assert!(!b2.intersect(&b1));
    }
}

impl Incrementable for usize {
    fn increment(&mut self) {
        *self += 1;
    }
}
