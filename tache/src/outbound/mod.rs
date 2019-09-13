mod direct;
mod fallback;
mod socks5;

pub trait Outbound {
    fn name(&self) -> String;
    fn udp(&self) -> bool;
    fn dial(&self) -> Result<(), String>;
    fn alive(&self) -> bool;
}
