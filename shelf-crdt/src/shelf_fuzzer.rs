use crate::wrap_crdt::Value;
use rand::{self, prelude::SliceRandom, rngs::StdRng, Rng, SeedableRng};
use serde_json::{self, json, Map, Number, Value as JSON};
use std::ops::Range;

const words: &'static [&'static str] = &[
    "Excepteur",
    "aliqua",
    "ullamco",
    "enim",
    "culpa",
    "sunt",
    "ad",
    "reprehenderit",
    "magna",
    "occaecat",
    "consequat",
    "pariatur",
    "quis",
    "esse",
    "voluptate",
    "anim",
    "Lorem",
    "non",
    "sed",
    "ea",
    "aute",
    "fugiat",
    "Duis",
    "exercitation",
    "dolor",
    "commodo",
    "minim",
    "veniam",
    "et",
    "consectetur",
    "adipiscing",
    "amet",
    "dolore",
    "officia",
    "cupidatat",
    "aliquip",
    "ipsum",
    "nisi",
    "cillum",
    "laborum",
    "nostrud",
    "irure",
    "Ut",
    "mollit",
    "ex",
    "qui",
    "eu",
    "ut",
    "tempor",
    "in",
    "labore",
    "velit",
    "do",
    "laboris",
    "elit",
    "id",
    "proident",
    "incididunt",
    "sint",
    "sit",
    "est",
    "deserunt",
    "eiusmod",
];

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
    pub fn generate_json_shelf(&mut self) -> JSON {
        return self.generate_children(1, true);
    }

    pub fn generate_json_values(&mut self) -> JSON {
        self.generate_children(1, false)
    }

    pub fn set_seed(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }

    fn generate_children(&mut self, depth: usize, include_clocks: bool) -> JSON {
        let mut children: Map<String, JSON> = Map::new();

        if depth <= self.rng.gen_range(self.depth_range.clone()) {
            let num_branches = self.rng.gen_range(self.branch_range.clone());
            let branches = (0..num_branches).map(|_| {
                (
                    self.random_string(),
                    self.generate_children(depth + 1, include_clocks),
                )
            });
            children.extend(branches);
        };
        let num_values = self.rng.gen_range(self.value_range.clone());
        let values = (0..num_values).map(|_| {
            let mut value = self.sample_value_recursive(depth);
            if include_clocks {
                value = self.wrap_in_clock(value, depth)
            }
            (self.random_string(), value)
        });
        children.extend(values);

        let children = JSON::Object(children);
        let clock: u16 =
            self.rng
                .gen_range((depth.checked_sub(2).unwrap_or(0))..(depth + 2)) as u16;
        json!([children, clock])
    }
    fn wrap_in_clock(&mut self, value: JSON, depth: usize) -> JSON {
        let clock = self
            .rng
            .gen_range((depth.checked_sub(2).unwrap_or(0))..(depth + 2)) as u16;

        json!([value, clock])
    }

    fn random_string(&mut self) -> String {
        words.choose(&mut self.rng).unwrap().to_string()
    }

    fn sample_value(&mut self) -> JSON {
        let pick: usize = self.rng.gen_range(0..2);

        match pick {
            0 => JSON::String(self.random_string()),
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
