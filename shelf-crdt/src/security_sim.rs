use crate::clock::{LamportTimestamp, SecureClock};
use crate::state_vector::StateVector;
use crate::wrap_crdt::Shelf;
use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::{Duration, SystemTime};

type SecureShelf = Shelf<usize, LamportTimestamp, SecureClock>;

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

*/

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
enum Message {
    StateVector(),
    Delta(),
    Terminate, // Kill this client process
}

#[derive(Clone, Serialize, Deserialize)]
struct Update {
    message: Message,
    from: String,
    timestamp: SystemTime,
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
    pub duration: usize,
}

struct Client {
    peers: HashMap<String, Sender<Message>>,
    inbox: Receiver<Message>,
    actions: Vec<Action>,
    client_config: ClientConfig,
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
        let clients: Vec<Self> = inboxes
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
                Self::new(peers, inbox)
            })
            .collect();

        clients
    }

    fn new_byzantine(peers: HashMap<String, Sender<Message>>, inbox: Receiver<Message>) -> Self {
        let client_config = ClientConfig::default();
        let actions = [].into_iter().collect();
        Self {
            peers,
            inbox,
            client_config,
            actions,
        }
    }

    fn new(peers: HashMap<String, Sender<Message>>, inbox: Receiver<Message>) -> Self {
        let client_config = ClientConfig::default();
        Self {
            peers,
            inbox,
            client_config,
            actions: vec![],
        }
    }
    fn run(mut self) {
        thread::spawn(move || loop {
            // Handle early quit
            self.step()
        });
    }

    fn step(&mut self) {
        let actions = &mut self.actions;
        let context: &mut ActionContext = self.as_mut();
        actions.iter_mut().for_each(|action| action.act(context))
    }
}

fn main() {}

enum Action {
    CheckInbox(ClientActions::CheckInbox),
}

impl Action {
    pub fn act(&mut self, context: &mut ActionContext) {
        match self {
            Action::CheckInbox(a) => a.act(context),
        }
    }
}

struct ActionContext<'a> {
    peers: &'a mut HashMap<String, Sender<Message>>,
    inbox: &'a mut Receiver<Message>,
    client_config: &'a mut ClientConfig,
}

impl<'a> AsMut<ActionContext<'a>> for Client {
    fn as_mut(&mut self) -> &mut ActionContext<'a> {
        &mut ActionContext {
            peers: &mut self.peers,
            inbox: &mut &mut self.inbox,
            client_config: &mut self.client_config,
        }
    }
}

mod ClientActions {
    use std::time::{Duration, SystemTime};

    use super::ActionContext;

    pub struct CheckInbox {
        interval: Duration,
        last_performed: SystemTime,
    }

    impl CheckInbox {
        fn should_run(&self) -> bool {
            SystemTime::now()
                .duration_since(self.last_performed)
                .map(|dur| dur > self.interval)
                .unwrap_or(false)
        }

        pub fn act(&mut self, client: &mut ActionContext) {}
    }
}
