pub trait Proxy {
    fn name(&self) -> &'static str;
    fn udp(&self) -> bool;
    fn dial(&self) -> Result<(), &'static str>;
    fn alice(&self) -> bool;
}
