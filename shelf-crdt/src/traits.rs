use crate::wrap_crdt::Shelf;
use bincode::{self, ErrorKind};
// pub mod temporal;
// pub mod wrap_crdt;
use serde::{de::DeserializeOwned, Serialize};
pub trait Incrementable {
    /// Increments the counter of the source type. It is expected that the type monotonically increases.
    fn increment(&mut self);
}

// Controls how types recursively merge together: TODO: should this be generic?
pub trait Mergeable<Other> {
    fn merge(self, other: Other) -> Self;
}

/// A type with an associated CRDT wrapper that a Doc can use to track/update the object
pub trait CRDTBackend: Clone {
    type Backend: DeltaCRDT;

    fn new_crdt(&self) -> Self::Backend;
}

pub trait TypeOrd {
    fn type_cmp(&self, other: &Self) -> std::cmp::Ordering;
}

pub trait DeltaCRDT {
    type Delta;
    type StateVector;
    fn get_state_vector(&self) -> Self::StateVector;
    fn get_state_delta(&self, sv: &Self::StateVector) -> Option<Self::Delta>;
}

pub trait UpdateStrategy {
    type Target;
    type Update;
    fn get_update(target: Self::Target);
    fn process_update(target: Self::Target, update: Self::Update);
    fn set(
        &mut self,
        target: &mut Self::Target,
        key: String,
        value: Self::Target,
    ) -> Option<Self::Target>;
}

pub trait ClockGenerator<Clock>
where
    Clock: PartialEq + PartialOrd,
{
    fn new_clock(&mut self) -> Clock;

    fn next_clock(&mut self, clock: Clock) -> Clock;
}
