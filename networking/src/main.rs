#![allow(unused_must_use)]
use std::{thread, time::Duration};

use networking::Multicast;
fn main() {
    let mut com1 = Multicast::new(1);
    let mut com2 = Multicast::new(2);

    let mut message = String::new();
    loop {
        println!("Send some text: ");
        std::io::stdin().read_line(&mut message);
        com1.send("foo", message.clone().as_bytes().into());
        thread::sleep(Duration::from_millis(100));
        message.clear();

        if let Some(message) = com2.try_recv() {
            let res = String::from_utf8(message.data).unwrap();
            println!("{}", res);
        }
    }
}
