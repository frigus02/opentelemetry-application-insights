use serde_repr::Serialize_repr;

/// Defines the level of severity for the event.
#[derive(Debug, Serialize_repr)]
#[repr(u8)]
pub(crate) enum SeverityLevel {
    Verbose = 0,
    Information = 1,
    Warning = 2,
    Error = 3,
    #[cfg(feature = "logs")]
    Critical = 4,
}
