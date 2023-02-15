use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::{self, Value as JSON};

use std::clone::Clone;
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::fmt::Debug;
use std::fmt::Display;
use std::hash::{Hash, Hasher};

use crate::traits::ClockGenerator;

// Gets the logical clock component of the clock
pub trait LogicalClock {
    fn get_logical_clock(&self) -> usize;
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct LamportTimestamp(pub usize);

impl LamportTimestamp {
    pub fn increment(self) -> Self {
        LamportTimestamp(self.0 + 1)
    }
}

impl LogicalClock for LamportTimestamp {
    fn get_logical_clock(&self) -> usize {
        self.0
    }
}

impl Default for LamportTimestamp {
    fn default() -> Self {
        Self(0)
    }
}

impl Display for LamportTimestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<usize> for LamportTimestamp {
    fn from(value: usize) -> Self {
        LamportTimestamp(value)
    }
}

impl From<LamportTimestamp> for JSON {
    fn from(value: LamportTimestamp) -> Self {
        let LamportTimestamp(clock) = value;
        JSON::Number(clock.into())
    }
}

impl TryFrom<JSON> for LamportTimestamp {
    type Error = String;

    fn try_from(value: JSON) -> Result<Self, Self::Error> {
        match value {
            JSON::Number(n) => n
                .as_u64()
                .map(|clock| (clock as usize).into())
                .ok_or_else(|| format!("Could not get usize for Lamport Timestamp")),
            v => Err(format!("Cannot parse Lamport Timestamp from {v}")),
        }
    }
}

pub struct LamportTimestampGenerator;

impl ClockGenerator for LamportTimestampGenerator {
    type Clock = LamportTimestamp;
    fn new_clock(&mut self) -> Self::Clock {
        LamportTimestamp(0)
    }

    fn next_clock(&mut self, clock: Self::Clock) -> Self::Clock {
        let LamportTimestamp(clock) = clock;
        LamportTimestamp(clock + 1)
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct DotClock {
    pub client_id: usize,
    pub clock: usize,
}

impl DotClock {
    pub fn new(client_id: usize) -> Self {
        DotClock {
            client_id,
            clock: 0,
        }
    }

    pub fn increment(&self, client_id: usize) -> Self {
        DotClock {
            client_id,
            clock: self.clock + 1,
        }
    }
}

impl LogicalClock for DotClock {
    fn get_logical_clock(&self) -> usize {
        self.clock
    }
}

impl Default for DotClock {
    fn default() -> Self {
        DotClock {
            client_id: 0,
            clock: 0,
        }
    }
}

impl PartialOrd for DotClock {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.clock.cmp(&other.clock) {
            order @ (Ordering::Greater | Ordering::Less) => Some(order),
            Ordering::Equal if self.client_id == other.client_id => Some(Ordering::Equal), // Ordering by client id is besides the point of shelf, but should we try it?
            _ => None,
        }
    }
}

impl PartialEq<LamportTimestamp> for DotClock {
    fn eq(&self, _: &LamportTimestamp) -> bool {
        false
    }
}

impl PartialEq<DotClock> for LamportTimestamp {
    fn eq(&self, _: &DotClock) -> bool {
        false
    }
}

impl PartialOrd<DotClock> for LamportTimestamp {
    fn partial_cmp(&self, other: &DotClock) -> Option<Ordering> {
        match self.0.partial_cmp(&other.clock) {
            Some(Ordering::Equal) => None,
            v => v,
        }
    }
}

impl PartialOrd<LamportTimestamp> for DotClock {
    fn partial_cmp(&self, other: &LamportTimestamp) -> Option<Ordering> {
        match self.clock.partial_cmp(&other.0) {
            Some(Ordering::Equal) => None,
            v => v,
        }
    }
}

impl Display for DotClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}, {}]", self.client_id, self.clock)
    }
}

impl TryFrom<JSON> for DotClock {
    type Error = String;

    fn try_from(value: JSON) -> Result<Self, Self::Error> {
        match value {
            JSON::Array(mut array) if array.len() == 2 => {
                match (array.remove(0), array.remove(0)) {
                    (JSON::Number(client_id), JSON::Number(clock)) => {
                        let client_id = client_id
                            .as_u64()
                            .ok_or(format!("Could not parse client_id from {client_id}"))?
                            as usize;
                        let clock = clock
                            .as_u64()
                            .ok_or(format!("Could not parse clock from {clock}"))?
                            as usize;
                        Ok(DotClock { client_id, clock })
                    }
                    v => Err(format!("Could not parse ShelfClock from {v:?}")),
                }
            }
            v => Err(format!("Could not extract ShelfClock from {v:?}")),
        }
    }
}

impl From<DotClock> for JSON {
    fn from(value: DotClock) -> Self {
        let DotClock { client_id, clock } = value;
        json!([client_id, clock])
    }
}

pub struct DotClockGenerator {
    client_id: usize,
}

impl DotClockGenerator {
    pub fn new(client_id: usize) -> Self {
        DotClockGenerator { client_id }
    }
}

impl ClockGenerator for DotClockGenerator {
    type Clock = DotClock;
    fn new_clock(&mut self) -> Self::Clock {
        DotClock {
            client_id: self.client_id,
            clock: 0,
        }
    }

    fn next_clock(&mut self, clock: Self::Clock) -> Self::Clock {
        let DotClock { clock, .. } = clock;
        DotClock {
            client_id: self.client_id,
            clock,
        }
    }
}

// NOTE: Fields are public for testing purposes, these should not be public in a deployed system.
#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SecureClock {
    pub clock: usize,
    pub hash: u64,
}

impl SecureClock {
    pub fn new<T: Hash>(value: &T, clock: usize) -> Self {
        let mut hasher = DefaultHasher::new(); // TODO use a different hasher
        let pair = (clock, value);
        pair.hash(&mut hasher);
        let hash = hasher.finish();
        Self { clock, hash }
    }

    pub fn verify(&self, value: &impl Hash) -> bool {
        let mut hasher = DefaultHasher::new(); // TODO use a different hasher
        let pair = (self.clock, value);
        pair.hash(&mut hasher);
        let hash = hasher.finish();
        self.hash == hash
    }

    pub fn next(&self, value: &impl Hash) -> Self {
        Self::new(&value, self.clock + 1)
    }
}

impl PartialOrd for SecureClock {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.clock.partial_cmp(&other.clock) {
            Some(core::cmp::Ordering::Equal) => {
                if self.hash == other.hash {
                    Some(core::cmp::Ordering::Equal)
                } else {
                    None
                }
            }

            ord => return ord,
        }
    }
}

impl LogicalClock for SecureClock {
    fn get_logical_clock(&self) -> usize {
        self.clock
    }
}

impl From<SecureClock> for JSON {
    fn from(value: SecureClock) -> Self {
        let SecureClock { hash, clock } = value;
        json!([hash, clock])
    }
}

impl TryFrom<JSON> for SecureClock {
    type Error = String;

    fn try_from(value: JSON) -> Result<Self, Self::Error> {
        match value {
            JSON::Array(mut array) if array.len() == 2 => {
                match (array.remove(0), array.remove(0)) {
                    (JSON::Number(hash), JSON::Number(clock)) => {
                        let hash = hash
                            .as_u64()
                            .ok_or(format!("Could not parse hash from {hash}"))?;
                        let clock = clock
                            .as_u64()
                            .ok_or(format!("Could not parse clock from {clock}"))?
                            as usize;
                        Ok(SecureClock { hash, clock })
                    }
                    v => Err(format!("Could not parse ShelfClock from {v:?}")),
                }
            }
            v => Err(format!("Could not extract ShelfClock from {v:?}")),
        }
    }
}

impl PartialEq<LamportTimestamp> for SecureClock {
    fn eq(&self, _: &LamportTimestamp) -> bool {
        false
    }
}

impl PartialEq<SecureClock> for LamportTimestamp {
    fn eq(&self, _: &SecureClock) -> bool {
        false
    }
}

impl PartialOrd<SecureClock> for LamportTimestamp {
    fn partial_cmp(&self, other: &SecureClock) -> Option<Ordering> {
        match self.0.partial_cmp(&other.clock) {
            Some(Ordering::Equal) => None,
            v => v,
        }
    }
}

impl PartialOrd<LamportTimestamp> for SecureClock {
    fn partial_cmp(&self, other: &LamportTimestamp) -> Option<Ordering> {
        match self.clock.partial_cmp(&other.0) {
            Some(Ordering::Equal) => None,
            v => v,
        }
    }
}

impl Display for SecureClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:x}, {}]", self.hash, self.clock)
    }
}

pub enum ShelfClock<'a, M, V> {
    MapClock(&'a M),
    ValueClock(&'a V),
}

impl<'a, M, V> LogicalClock for ShelfClock<'a, M, V>
where
    M: LogicalClock,
    V: LogicalClock,
{
    fn get_logical_clock(&self) -> usize {
        match &self {
            ShelfClock::MapClock(m) => m.get_logical_clock(),
            ShelfClock::ValueClock(v) => v.get_logical_clock(),
        }
    }
}

impl<'a, M, V> PartialEq for ShelfClock<'a, M, V>
where
    M: PartialEq + PartialEq<V>,
    V: PartialEq + PartialEq<M>,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::MapClock(l0), Self::MapClock(r0)) => l0 == r0,
            (Self::ValueClock(l0), Self::ValueClock(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl<'a, M, V> PartialOrd for ShelfClock<'a, M, V>
where
    M: PartialOrd + PartialOrd<V>,
    V: PartialOrd + PartialOrd<M>,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (ShelfClock::MapClock(l), ShelfClock::MapClock(r)) => l.partial_cmp(r),
            (ShelfClock::MapClock(l), ShelfClock::ValueClock(r)) => l.partial_cmp(r),
            (ShelfClock::ValueClock(l), ShelfClock::MapClock(r)) => l.partial_cmp(r),
            (ShelfClock::ValueClock(l), ShelfClock::ValueClock(r)) => l.partial_cmp(r),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_clock() {
        // Basic equality
        assert_eq!(SecureClock::new(&1, 5), SecureClock::new(&1, 5));

        // Gt/lt clock
        // Same val
        assert!(SecureClock::new(&1, 6) > SecureClock::new(&1, 5));
        assert!(SecureClock::new(&1, 5) < SecureClock::new(&1, 6));
        // Different val
        assert!(SecureClock::new(&2, 6) > SecureClock::new(&1, 5));
        assert!(SecureClock::new(&2, 5) < SecureClock::new(&1, 6));

        // Inequality with different content
        assert_ne!(SecureClock::new(&2, 6), SecureClock::new(&1, 6));
    }
}
