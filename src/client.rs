use dbus::{Connection, BusType, Message};

use common::DBUS_NAME;

pub fn run(args: &[&str]) {
    let conn = Connection::get_private(BusType::Session)
        .expect("Failed to connect DBUS");
    let message = Message::new_method_call(DBUS_NAME, "/", DBUS_NAME, "add")
        .unwrap()
        .append1(args);
    conn.send_with_reply_and_block(message, 1000)
        .expect("Failed to add item to the queue");
}

