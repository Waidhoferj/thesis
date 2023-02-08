use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
struct ORSet;
struct MVMap;

struct DotKernel<T: PartialEq> {
    context: DotContext,
    entries: HashMap<Dot, T>,
}

impl<T: PartialEq> DotKernel<T> {
    fn merge(self, other: DotKernel<T>) -> Self {
        let mut entries = self.entries;
        let entry_keys: HashSet<Dot> = entries.keys().cloned().collect();
        let other_entry_keys: Vec<Dot> = other.entries.keys().cloned().collect();
        // Add unseen info to this entries from other
        other
            .entries
            .into_iter()
            .filter(|(dot, _)| !(entry_keys.contains(dot) || self.context.contains(dot)))
            .for_each(|(dot, value)| {
                entries.insert(dot, value);
            });

        // Remove information that is within dot bounds but doesn't exist in other
        entry_keys
            .into_iter()
            .filter(|dot| other.context.contains(dot) && !other_entry_keys.contains(dot))
            .for_each(|dot| {
                entries.remove(&dot);
            });

        DotKernel {
            context: self.context.merge(other.context),
            entries,
        }
    }

    fn insert(&mut self, client_id: ClientId, value: T) {
        let dot = self.context.next_dot(client_id);
        self.entries.insert(dot, value);
    }

    fn remove(&mut self, value: T) {
        self.entries.retain(|_, entry| &value != entry)
    }
}

struct DotContext {
    integration_bound: VectorClock,
    detached_dots: HashSet<Dot>,
}

impl DotContext {
    fn contains(&self, dot: &Dot) -> bool {
        return self.integration_bound.contains(dot) || self.detached_dots.contains(dot);
    }

    fn next_dot(&mut self, client_id: ClientId) -> Dot {
        let clock_entry = self
            .integration_bound
            .entry(client_id)
            .and_modify(|e| *e += 1)
            .or_insert(0);
        let clock = *clock_entry;
        Dot { client_id, clock }
    }

    /// Combines the information in two dot contexts.
    fn merge(self, other: DotContext) -> Self {
        let dots: HashSet<Dot> = self
            .detached_dots
            .union(&other.detached_dots)
            .cloned()
            .collect();
        let integration_bound = self.integration_bound.merge(other.integration_bound);
        let merged_context = DotContext {
            integration_bound,
            detached_dots: dots,
        };
        merged_context.compact()
    }

    /// Garbage collects detached dots based on the integration bound.
    fn compact(self) -> Self {
        let mut dot_list: Vec<&Dot> = self.detached_dots.iter().collect();
        dot_list.sort_by_key(|dot| dot.clock);
        let mut integration_bound = self.integration_bound;
        let dots = dot_list
            .into_iter()
            .filter(|dot| {
                let is_touching_bound = integration_bound.integrate_dot(dot);
                !is_touching_bound || !integration_bound.contains(dot)
            })
            .cloned()
            .collect();
        DotContext {
            integration_bound,
            detached_dots: dots,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Dot {
    client_id: ClientId,
    clock: Clock,
}

impl PartialOrd for Dot {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.client_id.partial_cmp(&other.client_id) {
            Some(core::cmp::Ordering::Equal) => self.clock.partial_cmp(&other.clock),
            _ => None,
        }
    }
}

#[derive(PartialEq, Clone)]
struct VectorClock {
    clients: HashMap<ClientId, Clock>,
}

impl PartialOrd for VectorClock {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let mut order = self.clients.len().cmp(&other.clients.len());
        for (client_id, clock) in self.clients.iter() {
            if let Some(other_clock) = other.clients.get(client_id) {
                let clock_order = clock.cmp(other_clock);
                match (order, clock_order) {
                    (Ordering::Less, Ordering::Greater) | (Ordering::Greater, Ordering::Less) => {
                        return None
                    }
                    _ => (),
                };
                order = clock_order;
            }
        }
        return Some(order);
    }
}

impl VectorClock {
    fn contains(&self, dot: &Dot) -> bool {
        let Dot { client_id, clock } = dot;
        self.clients
            .get(client_id)
            .map(|v_clock| v_clock >= clock)
            .unwrap_or(false)
    }

    fn merge(self, other: VectorClock) -> Self {
        let mut clients = self.clients;
        for (client_id, clock) in other.clients.into_iter() {
            let clock_entry = clients.entry(client_id);
            clock_entry
                .and_modify(|c| *c = (*c).max(clock))
                .or_insert(clock);
        }
        VectorClock { clients }
    }

    fn entry(&mut self, client_id: ClientId) -> Entry<ClientId, Clock> {
        self.clients.entry(client_id)
    }

    fn integrate_dot(&mut self, dot: &Dot) -> bool {
        let Dot { client_id, clock } = dot;
        let clock_entry = self.clients.entry(*client_id);
        let integrated = if let Entry::Occupied(mut clock_entry) = clock_entry {
            let v_clock = *clock_entry.get();
            let integratable = v_clock + 1 == *clock;
            if integratable {
                clock_entry.insert(*clock);
            }
            return integratable || v_clock >= *clock;
        } else if clock == &0 {
            self.clients.insert(*client_id, 0);
            return true;
        } else {
            false
        };

        return integrated;
    }
}

type ClientId = isize;
type Clock = usize;

#[cfg(test)]
mod tests {
    use super::*;

    fn test_mvr() {
        unimplemented!()
    }

    fn test_orset() {
        unimplemented!()
    }
}
