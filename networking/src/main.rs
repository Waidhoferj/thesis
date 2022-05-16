#![allow(unused_must_use)]
use std::{collections::HashMap, thread, time::Duration};

use networking::Multicast;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
struct Message {
    text: String,
    other: HashMap<String, String>,
}

impl Message {
    fn new(text: String) -> Self {
        let mut other = HashMap::new();
        other.insert("foo".to_string(), "bar".to_string());
        other.insert("bar".to_string(), "foo".to_string());

        Message { text, other }
    }
}

fn main() {
    let mut com1 = Multicast::new(1);
    let mut com2 = Multicast::new(2);

    let mut message = String::new();
    loop {
        println!("Send some text: ");
        std::io::stdin().read_line(&mut message);
        com1.send(Message::new(message.clone()));
        thread::sleep(Duration::from_millis(100));
        message.clear();

        if let Some(message) = com2.try_recv::<Message>() {
            println!("{}, {:?}", message.text, message.other);
        }
    }
}
