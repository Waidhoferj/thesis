use rand::{self, prelude::SliceRandom, rngs::StdRng, seq::IteratorRandom, Rng, SeedableRng};
use random_word;
use serde_json::{self, json, Map, Number, Value as JSON};
use std::{collections::HashSet, ops::Range};

pub struct ShelfFuzzer {
    pub rng: StdRng,
    pub depth_range: Range<usize>,
    pub branch_range: Range<usize>,
    pub value_range: Range<usize>,
}

impl Default for ShelfFuzzer {
    fn default() -> Self {
        ShelfFuzzer::new(0)
    }
}

impl ShelfFuzzer {
    pub fn new(seed: u64) -> Self {
        ShelfFuzzer {
            rng: StdRng::seed_from_u64(seed),
            depth_range: 2..3,
            branch_range: 1..2,
            value_range: 0..1,
        }
    }
    pub fn generate_json_shelf(&mut self, client_id: usize) -> JSON {
        return self.generate_children(1, true, client_id);
    }

    pub fn generate_json_values(&mut self) -> JSON {
        self.generate_children(1, false, 0) // Clock value not used
    }

    pub fn set_seed(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }

    fn gen_keys(&mut self, n_keys: usize) -> Vec<String> {
        let mut keys = random_word::all()[..10000]
            .iter()
            .choose_multiple(&mut self.rng, n_keys);
        keys.shuffle(&mut self.rng);
        keys.into_iter().map(|s| s.to_string()).collect()
    }

    fn generate_children(&mut self, depth: usize, include_clocks: bool, client_id: usize) -> JSON {
        let mut children: Map<String, JSON> = Map::new();
        if depth <= self.rng.gen_range(self.depth_range.clone()) {
            let num_branches = self.rng.gen_range(self.branch_range.clone());
            let keys = self.gen_keys(num_branches);
            let branches = keys.into_iter().map(|key| {
                (
                    key,
                    self.generate_children(depth + 1, include_clocks, client_id),
                )
            });
            children.extend(branches);
        } else {
            let num_items = self.rng.gen_range(self.value_range.clone());
            let keys = self.gen_keys(num_items);
            let items = keys.into_iter().map(|key| {
                let mut value = self.sample_value_recursive(depth);
                if include_clocks {
                    value = self.wrap_in_value_clock(value, depth, client_id)
                }
                (key, value)
            });
            children.extend(items);
        }

        let children = JSON::Object(children);
        if include_clocks {
            self.wrap_in_map_clock(children, depth)
        } else {
            children
        }
    }
    fn wrap_in_value_clock(&mut self, value: JSON, depth: usize, client_id: usize) -> JSON {
        let clock = self
            .rng
            .gen_range((depth.checked_sub(2).unwrap_or(0))..(depth + 2)) as u16;

        json!([value, [client_id, clock]])
    }

    fn wrap_in_map_clock(&mut self, value: JSON, depth: usize) -> JSON {
        let clock = self
            .rng
            .gen_range((depth.checked_sub(2).unwrap_or(0))..(depth + 2)) as u16;

        json!([value, clock])
    }

    fn sample_value(&mut self) -> JSON {
        let pick: usize = self.rng.gen_range(0..2);

        match pick {
            0 => JSON::String(random_word::gen().to_string()),
            1 => JSON::Number(Number::from_f64(self.rng.gen()).unwrap()),
            _ => JSON::Bool(self.rng.gen()),
        }
    }

    fn sample_value_recursive(&mut self, depth: usize) -> JSON {
        let pick: usize = self.rng.gen_range(0..4);
        let array_size_range = 0..5;

        let value = match pick {
            0 => {
                let size: usize = self.rng.gen_range(array_size_range);
                let arr = if depth <= self.rng.gen_range(self.depth_range.clone()) {
                    (0..size)
                        .map(|_| self.sample_value_recursive(depth + 1))
                        .collect()
                } else {
                    (0..size).map(|_| self.sample_value()).collect()
                };

                JSON::Array(arr)
            }
            _ => self.sample_value(),
        };
        return value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn has_difference(this: &Map<String, JSON>, other: &Map<String, JSON>) -> bool {
        let this_keys: HashSet<&String> = this.keys().into_iter().collect();
        let other_keys: HashSet<&String> = other.keys().into_iter().collect();
        this_keys
            .difference(&other_keys)
            .into_iter()
            .next()
            .is_some()
            || other_keys
                .difference(&this_keys)
                .into_iter()
                .next()
                .is_some()
            || this_keys
                .into_iter()
                .any(|key| match (this.get(key), other.get(key)) {
                    (Some(JSON::Object(m1)), Some(JSON::Object(m2))) => has_difference(m1, m2),
                    _ => false,
                })
    }

    #[test]
    fn test_flat_json_generation() {
        let mut fuzzer = ShelfFuzzer {
            rng: StdRng::seed_from_u64(42),
            depth_range: 0..1,
            branch_range: 0..5,
            value_range: 1000..1001,
        };
        for _ in 0..50 {
            let json = fuzzer.generate_json_values();
            let obj = json.as_object().unwrap();
            assert_eq!(obj.len(), 1000);
            assert!(obj.iter().all(|(_, v)| !v.is_object()))
        }
    }
    #[test]
    fn test_subset() {
        let mut fuzzer = ShelfFuzzer {
            rng: StdRng::seed_from_u64(42),
            depth_range: 0..5,
            branch_range: 0..4,
            value_range: 1..2,
        };

        for _ in 0..50 {
            let json1 = fuzzer.generate_json_values();
            let json2 = fuzzer.generate_json_values();
            let obj1 = json1.as_object().unwrap();
            let obj2 = json2.as_object().unwrap();

            if !has_difference(obj1, obj2) {
                println!("oof")
            }
        }
    }
}
