use super::base::{PostCompatibleProtocol, TextProtocol};
use super::tbip::TbipClient;
use super::tmutp::TmutpClient;

pub fn get_protocol(scheme: &str) -> Option<Box<dyn TextProtocol>> {
    match scheme.to_ascii_lowercase().as_str() {
        "tmutp" => Some(Box::new(TmutpClient)),
        "tbip" => Some(Box::new(TbipClient)),
        _ => None,
    }
}

#[allow(unused)]
pub fn get_post_protocol(scheme: &str) -> Option<Box<dyn PostCompatibleProtocol>> {
    match scheme.to_ascii_lowercase().as_str() {
        "tmutp" => Some(Box::new(TmutpClient)),
        _ => None,
    }
}