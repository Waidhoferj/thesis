use shelf_crdt::traits::{CRDTBackend, DeltaCRDT, Mergeable};
use shelf_crdt_macros::CRDT;

#[derive(Clone, CRDT)]
struct MyData {
    name: String,
    fav_num: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_use() {
        // Init data
        let mut my_data = MyData {
            name: "John".to_string(),
            fav_num: 7,
        };
        // Integrate with crdt
        let mut crdt = my_data.new_crdt();
        let mut crdt2 = my_data.new_crdt();

        // Make changes
        my_data.name = "Jason".to_string();
        my_data.fav_num = 20;

        // Update the crdt
        crdt.update(&my_data);

        // sync data
        let sv = crdt2.get_state_vector();
        let delta = crdt.get_state_delta(&sv).unwrap();
        crdt2.merge(delta);

        assert_eq!(&crdt2.state.name, "Jason");
        assert_eq!(crdt2.state.fav_num, 20);
    }
    #[test]
    fn partial_update() {
        // Init data
        let mut my_data = MyData {
            name: "John".to_string(),
            fav_num: 7,
        };
        let mut data2 = my_data.clone();
        // Integrate with crdt
        let mut crdt = my_data.new_crdt();
        let mut crdt2 = my_data.new_crdt();

        // Make changes to crdt1
        my_data.fav_num = 20;

        // Make changes to crdt2
        data2.name = "Alfred".to_string();

        // Update the crdt
        crdt.update(&my_data);
        crdt2.update(&data2);

        // sync data
        let sv = crdt.get_state_vector();
        let sv2 = crdt2.get_state_vector();

        let delta = crdt.get_state_delta(&sv2).unwrap();
        let delta2 = crdt2.get_state_delta(&sv).unwrap();

        crdt2.merge(delta);
        crdt.merge(delta2);

        assert_eq!(&crdt2.state.name, &crdt.state.name,);
        assert_eq!(crdt2.state.fav_num, crdt.state.fav_num);
    }
}
