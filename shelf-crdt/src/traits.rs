use std::ops::Deref;

// pub mod temporal;
// pub mod wrap_crdt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
pub trait Incrementable {
    /// Increments the counter of the source type. It is expected that the type monotonically increases.
    fn increment(&mut self);
}

// Controls how types recursively merge together: TODO: should this be generic?
pub trait Mergeable<Other> {
    fn merge(&mut self, other: Other);
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
    type Delta: Serialize + DeserializeOwned;
    type StateVector: Serialize + DeserializeOwned;
    fn get_state_vector(&self) -> Self::StateVector;
    fn get_state_delta(&self, sv: &Self::StateVector) -> Option<Self::Delta>;
}
