
use regex::Regex; 

pub const DBUS_INTERFACE: &'static str = "org.ProcessQueue";

#[derive(Debug)]
pub struct NameError;

pub fn get_dbus_name(name: &str) -> Result<String, NameError> {
    let regex = Regex::new(r"^[a-zA-Z]+$").unwrap();
    if regex.is_match(name) {
        Ok(format!("org.ProcessQueue.{}", name))
    } else {
        Err(NameError)
    }
}

