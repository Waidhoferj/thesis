use crate::clock::{LamportTimestamp, LogicalClock, SecureClock};
use crate::json::Value;
use crate::state_vector::StateVector;
use crate::traits::DeltaCRDT;
use crate::wrap_crdt::Shelf;

use rand::seq::{IteratorRandom, SliceRandom};
use rand::{self, Rng};
use random_word;
use std::collections::HashMap;
use std::sync::mpsc::{self, channel};
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

type SecureShelf = Shelf<usize, LamportTimestamp, SecureClock>;
type SecureStateVector = StateVector<LamportTimestamp, SecureClock>;

/*
   Interface:
       Client: Represents a user of the CRDT. Has some strategy for producing updates.
           - List of peers to send data to
           - Message type that can send:
               - State vectors
               - Deltas
               - Metadata messages
       Manager: Handles Logging, cross-client metrics and eventual convergence

   Tests:
       1. CRDT converges even with malicious nodes
       2. Throughput
       3. Latency
       4. Dropped info
       5. Node coming online after a while

   Plan:
       1. Client
    Valid Actions:
        1. Check Inbox
        2. Random shelf edit
        3. Send updates
    Byzantine actions:
        1. Corrupt clock (inc by 1)
        2. Corrupt value (randomly replace)


*/

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
enum Payload {
    StateVector(SecureStateVector),
    Delta(SecureShelf),
    Terminate, // Kill this client process
}

#[derive(Clone, Serialize, Deserialize)]
struct Message {
    payload: Payload,
    from: String,
    timestamp: SystemTime,
}

impl Message {
    pub fn new(from: String, payload: Payload) -> Self {
        Self {
            from,
            payload,
            timestamp: SystemTime::now(),
        }
    }
}

struct ClientConfig {
    update_interval: Duration,
    set_interval: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            update_interval: Duration::from_millis(10),
            set_interval: Duration::from_millis(1),
        }
    }
}

#[derive(Clone, Copy)]
struct SimulationConfig {
    pub n_nodes: usize,
    pub p_byzantine: f64,
    pub duration: Duration,
}

struct Client {
    uid: String,
    peers: HashMap<String, Sender<Message>>,
    inbox: Receiver<Message>,
    actions: Vec<ClientAction>,
    shelf: SecureShelf,
}

impl Client {
    fn new_network(config: &SimulationConfig) -> Vec<Self> {
        let (outboxes, inboxes): (Vec<Sender<Message>>, Vec<Receiver<Message>>) =
            (0..config.n_nodes).fold((vec![], vec![]), |(mut outboxes, mut inboxes), _| {
                let (tx, rx) = channel();
                inboxes.push(rx);
                outboxes.push(tx);
                (outboxes, inboxes)
            });
        let byzantine_split = config.p_byzantine * config.n_nodes as f64;
        let byzantine_split = byzantine_split.floor() as usize;
        let mut clients: Vec<Self> = inboxes
            .into_iter()
            .enumerate()
            .map(|(uid, inbox)| {
                let peers: HashMap<String, _> = outboxes
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(peer_uid, address)| {
                        let peer_uid = peer_uid.to_string();
                        (peer_uid, address)
                    })
                    .collect();
                if uid < byzantine_split {
                    Self::new_byzantine(uid.to_string(), inbox, peers)
                } else {
                    Self::new(uid.to_string(), inbox, peers)
                }
            })
            .collect();

        // Randomize order
        clients.shuffle(&mut rand::thread_rng());
        clients
    }

    fn new_byzantine(
        uid: String,
        inbox: Receiver<Message>,
        peers: HashMap<String, Sender<Message>>,
    ) -> Self {
        let mut rng = rand::thread_rng();
        let actions = [
            ClientAction::new(
                Action::CheckInbox,
                Duration::from_millis(rng.gen_range(1..20)),
            ),
            ClientAction::new(
                Action::SendUpdate,
                Duration::from_millis(rng.gen_range(1..20)),
            ),
            ClientAction::new(
                Action::RandomEdit,
                Duration::from_millis(rng.gen_range(5..10)),
            ),
            ClientAction::new(
                Action::CorruptClock,
                Duration::from_millis(rng.gen_range(5..10)),
            ),
            ClientAction::new(
                Action::CorruptValue,
                Duration::from_millis(rng.gen_range(5..10)),
            ),
        ];
        Self::from_actions(uid, inbox, peers, actions)
    }

    fn new(uid: String, inbox: Receiver<Message>, peers: HashMap<String, Sender<Message>>) -> Self {
        let mut rng = rand::thread_rng();
        let actions = [
            ClientAction::new(
                Action::CheckInbox,
                Duration::from_millis(rng.gen_range(1..20)),
            ),
            ClientAction::new(
                Action::SendUpdate,
                Duration::from_millis(rng.gen_range(1..20)),
            ),
            ClientAction::new(
                Action::RandomEdit,
                Duration::from_millis(rng.gen_range(5..10)),
            ),
            ClientAction::new(Action::CheckForCorruption, Duration::from_millis(0)),
        ];
        Self::from_actions(uid, inbox, peers, actions)
    }

    fn from_actions(
        uid: String,
        inbox: Receiver<Message>,
        peers: HashMap<String, Sender<Message>>,
        actions: impl IntoIterator<Item = ClientAction>,
    ) -> Self {
        Self {
            uid,
            peers,
            inbox,
            actions: actions.into_iter().collect(),
            shelf: Shelf::Map {
                shelves: HashMap::new(),
                clock: 0.into(),
            },
        }
    }

    pub fn step(&mut self) -> Option<SecureShelf> {
        let actions = &mut self.actions;
        let mut context: ActionContext = ActionContext {
            uid: self.uid.as_str(),
            peers: &self.peers,
            inbox: &mut self.inbox,
            shelf: &mut self.shelf,
        };
        let follow_ups: Vec<Action> = actions
            .iter_mut()
            .filter(|action| action.should_run())
            .filter_map(|action| action.act(&mut context))
            .collect();
        for action in follow_ups {
            if let Action::Terminate = action {
                return Some(self.shelf.clone());
            }
        }
        None
    }

    pub fn is_byzantine(&self) -> bool {
        self.actions
            .iter()
            .any(|action| matches!(action.action, Action::CorruptClock | Action::CorruptValue))
    }
}

struct ClientAction {
    interval: Duration,
    last_performed: SystemTime,
    action: Action,
}

impl ClientAction {
    pub fn new(action: Action, interval: Duration) -> Self {
        Self {
            interval,
            action,
            last_performed: std::time::UNIX_EPOCH,
        }
    }
    pub fn should_run(&self) -> bool {
        SystemTime::now()
            .duration_since(self.last_performed)
            .map(|dur| dur > self.interval)
            .unwrap_or(false)
    }
    pub fn act(&mut self, context: &mut ActionContext) -> Option<Action> {
        let follow_up_action = self.action.act(context);
        self.last_performed = SystemTime::now();
        follow_up_action
    }
}

enum Action {
    CheckInbox,
    RandomEdit,
    SendUpdate,
    CorruptClock,
    CorruptValue,
    CheckForCorruption,
    Terminate,
}

impl Action {
    pub fn act(&mut self, context: &mut ActionContext) -> Option<Action> {
        match self {
            Action::CheckInbox => return self.check_inbox(context),
            Action::RandomEdit => {
                self.random_edit(context);
            }
            Action::SendUpdate => {
                self.send_update(context);
            }
            Action::CorruptClock => {
                self.corrupt_clock(context);
            }
            Action::CorruptValue => {
                self.corrupt_value(context);
            }
            Action::CheckForCorruption => {
                self.check_corruption(context);
            }
            Action::Terminate => return Some(Action::Terminate),
        }
        None
    }

    fn check_inbox(&mut self, context: &mut ActionContext) -> Option<Action> {
        loop {
            let message = match context.inbox.recv_timeout(Duration::from_millis(5)) {
                Ok(message) => message,
                Err(mpsc::RecvTimeoutError::Timeout) => break,
                Err(err) => panic!("{}", err),
            };
            let res = self.process_message(message, context);
            if res.is_some() {
                return res;
            }
        }
        None
    }

    fn process_message(&mut self, message: Message, context: &mut ActionContext) -> Option<Action> {
        match message.payload {
            Payload::StateVector(sv) => {
                if let Some(delta) = context.shelf.get_state_delta(&sv) {
                    let outbox = context.peers.get(&message.from).unwrap();
                    let payload = Payload::Delta(delta);
                    let response = Message::new(context.uid.to_owned(), payload);
                    let send_res = outbox.send(response);
                }
                None
            }
            Payload::Delta(delta) => {
                let mut tmp = Shelf::Map {
                    shelves: HashMap::new(),
                    clock: 0.into(),
                };
                std::mem::swap(&mut tmp, context.shelf);
                *context.shelf = tmp.secure_merge(delta);
                None
            }
            Payload::Terminate => Some(Action::Terminate),
        }
    }
    fn random_edit(&self, context: &mut ActionContext) {
        let key = random_word::gen().to_owned();
        let value: usize = rand::thread_rng().gen();
        const SHELF_SIZE_LIMIT: usize = 200_000;
        match context.shelf {
            Shelf::Map {
                shelves,
                clock: parent_clock,
            } => {
                if shelves.len() < SHELF_SIZE_LIMIT {
                    // add a value
                    let clock = SecureClock::new(&value, parent_clock.0);
                    let shelf = Shelf::Value { value, clock };
                    shelves.insert(key, shelf);
                } else {
                    // remove some values
                    let keys = shelves
                        .keys()
                        .into_iter()
                        .cloned()
                        .choose_multiple(&mut rand::thread_rng(), SHELF_SIZE_LIMIT / 2);
                    for key in keys {
                        shelves.remove(&key);
                    }
                    shelves.values_mut().for_each(|shelf| match shelf {
                        Shelf::Value { value, clock } => {
                            *clock = SecureClock::new(value, clock.get_logical_clock() + 1)
                        }
                        Shelf::Map { .. } => {
                            unreachable!("Data structure should be flat.")
                        }
                    });
                    parent_clock.0 += 1;
                }
            }
            Shelf::Value { .. } => unreachable!("Top level shelves should be dicts"),
        }
    }
    fn send_update(&self, context: &mut ActionContext) {
        // Create state vector
        let sv = context.shelf.get_state_vector();
        // Multicast to all peers
        context
            .peers
            .iter()
            .filter(|(peer_id, _)| peer_id.as_str() != context.uid)
            .for_each(|(_, outbox)| {
                let payload = Payload::StateVector(sv.clone());

                let send_res = outbox.send(Message::new(context.uid.to_owned(), payload));
            })
    }

    fn corrupt_clock(&self, context: &mut ActionContext) {
        match context.shelf {
            Shelf::Value { .. } => unreachable!("Top level is a map"),
            Shelf::Map { shelves, .. } => {
                if shelves.is_empty() {
                    return;
                }

                if let Some((key, Shelf::Value { clock, .. })) =
                    shelves.iter_mut().choose(&mut rand::thread_rng())
                {
                    clock.clock = rand::random();
                } else {
                    unreachable!("Shelf should be flat")
                }
            }
        }
    }

    fn corrupt_value(&self, context: &mut ActionContext) {
        match context.shelf {
            Shelf::Value { .. } => unreachable!("Top level is a map"),
            Shelf::Map { shelves, .. } => {
                if shelves.is_empty() {
                    return;
                }

                if let Some((key, Shelf::Value { value, .. })) =
                    shelves.iter_mut().choose(&mut rand::thread_rng())
                {
                    *value = rand::random();
                } else {
                    unreachable!("Shelf should be flat")
                }
            }
        }
    }

    fn check_corruption(&self, context: &mut ActionContext) {
        if let Shelf::Map { shelves, .. } = context.shelf {
            for (key, sub_shelf) in shelves.iter() {
                if let Shelf::Value { clock, value } = sub_shelf {
                    assert!(
                        clock.verify(&value),
                        "Found corrupt pair in shelf: {key}: [{value}, {clock}]"
                    );
                } else {
                    unreachable!("Should be flat structure")
                }
            }
        } else {
            unreachable!("Top level should be map")
        }
    }
}

struct ActionContext<'a> {
    uid: &'a str,
    peers: &'a HashMap<String, Sender<Message>>,
    inbox: &'a mut Receiver<Message>,
    shelf: &'a mut SecureShelf,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn simulate_byzantine_network() {
        let config = SimulationConfig {
            n_nodes: 10,
            p_byzantine: 0.4,
            duration: Duration::from_secs(10),
        };
        let network = Client::new_network(&config);
        let mailboxes = network[0].peers.clone();
        let handles: Vec<_> = network
            .into_iter()
            .map(|mut client| {
                thread::spawn(move || loop {
                    if let Some(shelf) = client.step() {
                        return (shelf, client.is_byzantine());
                    }
                })
            })
            .collect();
        thread::sleep(config.duration);
        mailboxes.values().for_each(|mailbox| {
            mailbox
                .send(Message::new("".to_owned(), Payload::Terminate))
                .unwrap()
        });
        let shelf_results = handles.into_iter().map(|h| h.join().unwrap());
        let valid_shelves = shelf_results
            .filter(|(_, is_byzantine)| !is_byzantine)
            .map(|(shelf, _)| shelf);
        for shelf in valid_shelves {
            if let Shelf::Map { shelves, .. } = shelf {
                for sub_shelf in shelves.into_values() {
                    if let Shelf::Value { clock, value } = sub_shelf {
                        assert!(clock.verify(&value))
                    } else {
                        unreachable!("Should be flat structure")
                    }
                }
            } else {
                unreachable!("Top level should be map")
            }
        }
    }

    #[test]
    fn simulate_network_sequential() {
        const STEPS: usize = 10;
        let config = SimulationConfig {
            n_nodes: 4,
            p_byzantine: 0.5,
            duration: Duration::from_secs(5),
        };
        let mut network = Client::new_network(&config);
        for i in 0..STEPS {
            network.iter_mut().for_each(|client| {
                client.step();
            })
        }

        let valid_clients = network.into_iter().filter(|client| !client.is_byzantine());
        for client in valid_clients {
            let shelf = client.shelf;
            if let Shelf::Map { shelves, .. } = shelf {
                for sub_shelf in shelves.into_values() {
                    if let Shelf::Value { clock, value } = sub_shelf {
                        assert!(clock.verify(&value))
                    } else {
                        unreachable!("Should be flat structure")
                    }
                }
            } else {
                unreachable!("Top level should be map")
            }
        }
    }
}
