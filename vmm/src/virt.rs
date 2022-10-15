pub trait Virtualizable {
    fn is_supported(&self) -> bool;
    fn enable(&mut self) -> Result<(), VirtualizationError>;
    fn disable(&mut self) -> Result<(), VirtualizationError>;
}

pub enum VirtualizationError {
    NotSupported,
}
