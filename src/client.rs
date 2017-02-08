use std;
use dbus::{Connection, BusType, Message};

use common::{dbus_get_name, dbus_name_exists,
             DBUS_INTERFACE, DBUS_METHOD_ADD, DBUS_METHOD_STOP};

pub fn send(name: &str, args: &[&str]) {
    let full_name = dbus_get_name(name).expect("invalid server name");
    let conn = Connection::get_private(BusType::Session)
        .expect("failed to connect DBus");
    check_name(&conn, name, &full_name);
    let message = Message::new_method_call(full_name, "/", DBUS_INTERFACE, DBUS_METHOD_ADD)
        .unwrap()
        .append1(args);
    conn.send_with_reply_and_block(message, 1000)
        .expect("failed to add item to the queue");
}

pub fn stop(name: &str) {
    let full_name = dbus_get_name(name).expect("invalid server name");
    let conn = Connection::get_private(BusType::Session)
        .expect("failed to connect DBus");
    check_name(&conn, name, &full_name);
    let message = Message::new_method_call(full_name, "/", DBUS_INTERFACE, DBUS_METHOD_STOP)
        .unwrap();
    conn.send_with_reply_and_block(message, 1000)
        .expect("failed to stop the server");
}

pub fn has_server(name: &str) {
    let full_name = dbus_get_name(name).expect("invalid server name");
    let conn = Connection::get_private(BusType::Session)
        .expect("failed to connect DBus");
    let mut status = 0;
    if dbus_name_exists(&conn, &full_name)
        .expect("failed to check if the name exists") {
        println!("server \"{}\" exists", name);
    } else {
        println!("server \"{}\" does not exists", name);
        status = 1;
    }
    std::process::exit(status);
}

fn check_name(connection: &Connection, short_name: &str, full_name: &str) {
    if !dbus_name_exists(&connection, full_name)
        .expect("failed to check if the name exists") {
        error!("server \"{}\" does not exists", short_name);
        std::process::exit(1);
    }
}
