use crate::traits::Mergeable;
use bincode;
use networking::Multicast;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Deref};

use crate::traits::{CRDTBackend, DeltaCRDT};

pub struct Doc<T: DeltaCRDT> {
    pub elements: HashMap<String, T>, // Eventually map to Box<dyn CRDT>
    pub communicator: Multicast,
}

impl<T: DeltaCRDT + Default> Default for Doc<T> {
    fn default() -> Self {
        let mut rng = rand::thread_rng();
        Doc {
            elements: HashMap::new(),
            communicator: Multicast::new(rng.gen()),
        }
    }
}

pub trait DocElement<T: DeltaCRDT>:
    DeltaCRDT + Mergeable<T::Delta> + Serialize + Default + std::marker::Sized
{
}

impl<CRDT: DeltaCRDT + Mergeable<CRDT::Delta> + Serialize + Default + Deref> Doc<CRDT> {
    pub fn register<D: CRDTBackend<Backend = CRDT> + Default>(&mut self, id: String, data: D) {
        let crdt = data.new_crdt();
        self.elements.insert(id, crdt);
    }

    pub fn get<'a>(&'a self, id: &str) -> &'a CRDT::Target {
        self.elements.get(id).unwrap().deref()
    }

    pub fn update<D: CRDTBackend<Backend = CRDT>>(
        &mut self,
        id: &str,
        data: &D,
    ) -> Result<(), String>
    where
        CRDT: Mergeable<D>,
    {
        // Pull in updates
        self.apply_updates()?;
        // find diff and update SV
        if let Some(crdt) = self.elements.get_mut(id) {
            crdt.merge(data.clone());
        }

        // Send off SV
        self.sync();
        Ok(())
    }

    pub fn sync(&mut self) {
        let sv = self.get_state_vector();
        let message = DocMessage::<CRDT>::StateVector {
            clocks: sv,
            sender: self.communicator.id,
        };
        self.communicator.send(message)
    }

    pub fn apply_updates(&mut self) -> Result<(), String> {
        while let Some(message) = self.communicator.try_recv() {
            match message {
                DocMessage::<CRDT>::StateVector { clocks, sender }
                    if self.communicator.id != sender =>
                {
                    let delta = self.get_state_delta(&clocks).unwrap();
                    let message = DocMessage::<CRDT>::Delta {
                        diff: delta,
                        recipient: sender,
                    };
                    self.communicator.send(message);
                }

                DocMessage::<CRDT>::Delta { diff, recipient }
                    if recipient == self.communicator.id =>
                {
                    self.merge(diff)
                }
                _ => (),
            };
        }
        Ok(())
    }
}
impl<T: DeltaCRDT + Mergeable<T::Delta> + Serialize + Default> Mergeable<HashMap<String, T::Delta>>
    for Doc<T>
{
    fn merge(&mut self, delta_doc: HashMap<String, T::Delta>) {
        for (k, delta) in delta_doc {
            let crdt = self.elements.entry(k).or_default();
            crdt.merge(delta);
        }
    }
}

impl<T: DeltaCRDT + Mergeable<T::Delta> + Serialize + Default> DeltaCRDT for Doc<T> {
    type Delta = HashMap<String, T::Delta>;

    type StateVector = HashMap<String, T::StateVector>;

    fn get_state_vector(&self) -> Self::StateVector {
        self.elements
            .iter()
            .map(|(k, crdt)| (k.clone(), crdt.get_state_vector()))
            .collect()
    }

    fn get_state_delta(&self, sv: &Self::StateVector) -> Option<Self::Delta> {
        let mut doc_delta = Self::Delta::default();
        let default_sv = T::default().get_state_vector();
        for (k, crdt) in self.elements.iter() {
            let state_vec = sv.get(k).unwrap_or(&default_sv);
            if let Some(delta) = crdt.get_state_delta(state_vec) {
                doc_delta.insert(k.clone(), delta);
            }
        }

        Some(doc_delta)
    }
}

impl<T: DeltaCRDT + Mergeable<T::Delta> + Serialize + Default> Deref for Doc<T> {
    type Target = HashMap<String, T>;

    fn deref(&self) -> &Self::Target {
        &self.elements
    }
}

#[derive(Serialize, Deserialize, Clone)]
enum DocMessage<T: DeltaCRDT> {
    StateVector {
        clocks: HashMap<String, T::StateVector>,
        sender: u8,
    },
    Delta {
        diff: HashMap<String, T::Delta>,
        recipient: u8,
    },
}
