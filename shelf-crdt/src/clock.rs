use serde::{Deserialize, Serialize};
use serde_json::{self, Value as JSON};

use std::clone::Clone;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::fmt::Display;

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub struct LamportTimestamp(pub usize);

impl LamportTimestamp {
    pub fn increment(self) -> Self {
        LamportTimestamp(self.0 + 1)
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

impl TryFrom<LamportTimestamp> for JSON {
    type Error = ();

    fn try_from(value: LamportTimestamp) -> Result<Self, Self::Error> {
        let LamportTimestamp(clock) = value;
        Ok(JSON::Number(clock.into()))
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

#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ClientClock {
    pub client_id: usize,
    pub clock: usize,
}

impl ClientClock {
    pub fn new(client_id: usize) -> Self {
        ClientClock {
            client_id,
            clock: 0,
        }
    }

    pub fn increment(&self, client_id: usize) -> Self {
        ClientClock {
            client_id,
            clock: self.clock + 1,
        }
    }
}

impl Default for ClientClock {
    fn default() -> Self {
        ClientClock {
            client_id: 0,
            clock: 0,
        }
    }
}

impl PartialOrd for ClientClock {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.clock.cmp(&other.clock) {
            order @ (Ordering::Greater | Ordering::Less) => Some(order),
            Ordering::Equal if self.client_id == other.client_id => Some(Ordering::Equal), // Ordering by client id is besides the point of shelf, but should we try it?
            _ => None,
        }
    }
}

impl PartialEq<LamportTimestamp> for ClientClock {
    fn eq(&self, _: &LamportTimestamp) -> bool {
        false
    }
}

impl PartialEq<ClientClock> for LamportTimestamp {
    fn eq(&self, _: &ClientClock) -> bool {
        false
    }
}

impl PartialOrd<ClientClock> for LamportTimestamp {
    fn partial_cmp(&self, other: &ClientClock) -> Option<Ordering> {
        match self.0.partial_cmp(&other.clock) {
            Some(Ordering::Equal) => None,
            v => v,
        }
    }
}

impl PartialOrd<LamportTimestamp> for ClientClock {
    fn partial_cmp(&self, other: &LamportTimestamp) -> Option<Ordering> {
        match self.clock.partial_cmp(&other.0) {
            Some(Ordering::Equal) => None,
            v => v,
        }
    }
}

impl Display for ClientClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}, {}]", self.client_id, self.clock)
    }
}

impl TryFrom<JSON> for ClientClock {
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
                        Ok(ClientClock { client_id, clock })
                    }
                    v => Err(format!("Could not parse ShelfClock from {v:?}")),
                }
            }
            v => Err(format!("Could not extract ShelfClock from {v:?}")),
        }
    }
}

pub enum ShelfClock<'a, M, V> {
    MapClock(&'a M),
    ValueClock(&'a V),
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
