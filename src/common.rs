
pub const DBUS_INTERFACE: &'static str = "org.ProcessQueue";

pub fn get_dbus_name(name: &str) -> String {
    format!("org.ProcessQueue.{}", name)
}

