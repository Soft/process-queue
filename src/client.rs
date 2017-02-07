use dbus::{Connection, BusType, Message};

use common::{get_dbus_name, DBUS_INTERFACE};

pub fn run(name: &str, args: &[&str]) {
    let name = get_dbus_name(name).expect("invalid server name");
    let conn = Connection::get_private(BusType::Session)
        .expect("failed to connect DBus");
    let message = Message::new_method_call(name, "/", DBUS_INTERFACE, "add")
        .unwrap()
        .append1(args);
    conn.send_with_reply_and_block(message, 1000)
        .expect("failed to add item to the queue");
}

