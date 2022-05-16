#![allow(unused_must_use)]

extern crate nanomsg;

use nanomsg::{Error, Protocol, Socket};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use std::thread;

use std::io::Write;

const CLIENT_DEVICE_URL: &'static str = "ipc:///tmp/thesis_multicast_client.ipc";
const SERVER_DEVICE_URL: &'static str = "ipc:///tmp/thesis_multicast_server.ipc";
const TOPIC: &'static str = "topic";

pub struct Multicast {
    pub id: u8,
    write_socket: Socket,
    read_socket: Socket,
}

impl Multicast {
    pub fn new(id: u8) -> Self {
        let mut write_socket = Socket::new(Protocol::Pub).unwrap();
        write_socket.connect(SERVER_DEVICE_URL).unwrap();

        let mut read_socket = Socket::new(Protocol::Sub).unwrap();
        read_socket.subscribe(TOPIC.as_bytes()).unwrap();
        read_socket.connect(CLIENT_DEVICE_URL).unwrap();
        thread::spawn(Self::connect_sockets);
        Multicast {
            id,
            write_socket,
            read_socket,
        }
    }

    pub fn try_recv<Message: Serialize + DeserializeOwned>(&mut self) -> Option<Message> {
        let mut msg = Vec::new();
        self.read_socket
            .nb_read_to_end(&mut msg)
            .ok()
            .and_then(|_| bincode::deserialize::<Message>(&msg[TOPIC.len()..]).ok())
    }

    pub fn send<Message: Serialize + DeserializeOwned>(&mut self, message: Message) {
        let mut msg = Vec::new();
        msg.clear();
        msg.extend_from_slice(TOPIC.as_bytes());
        // Lib0 Serde
        let message = bincode::serialize(&message).unwrap();
        msg.extend_from_slice(&message);
        self.write_socket.write_all(&msg).unwrap();
    }

    fn connect_sockets() -> Result<(), Error> {
        let mut front_socket = Socket::new_for_device(Protocol::Pub)?;
        let mut front_endpoint = front_socket.bind(CLIENT_DEVICE_URL)?;
        let mut back_socket = Socket::new_for_device(Protocol::Sub)?;
        back_socket.subscribe(TOPIC.as_bytes())?;
        let mut back_endpoint = back_socket.bind(SERVER_DEVICE_URL)?;
        Socket::device(&front_socket, &back_socket);
        front_endpoint.shutdown();
        back_endpoint.shutdown();
        Ok(())
    }
}

// #[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
// pub struct Message {
//     pub data: Vec<u8>,
//     pub sender: u8,
//     pub topic: String,
//     pub target: Option<u8>,
// }

#[cfg(test)]
mod test {
    use std::{
        thread::{self, sleep},
        time::Duration,
    };

    use serde::{Deserialize, Serialize};

    use crate::Multicast;
    #[derive(Serialize, Deserialize, Clone)]
    struct Message(isize);

    #[test]
    fn test_send_data() {
        let mut com1 = Multicast::new(1);
        let mut com2 = Multicast::new(2);
        let data = Message(1);
        com1.send(data.clone());
        thread::sleep(Duration::from_millis(1000));
        if let Some(res) = com2.try_recv::<Message>() {
            assert_eq!(res.0, data.0);
        } else {
            panic!("Did not find data");
        }
        sleep(Duration::from_millis(2000));
        if let Some(res) = com1.try_recv::<Message>() {
            panic!("Should not have received data: {:?}", res.0);
        }
    }
}
