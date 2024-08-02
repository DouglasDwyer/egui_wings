use wings::*;

/// Identifies the set of host systems that will be available to guests.
#[export_type]
pub struct ExampleHost;

/// The set of events this module raises.
pub mod on {
    use super::*;

    /// Raised when the host is rendering a frame.
    #[export_type]
    pub struct Render;
}