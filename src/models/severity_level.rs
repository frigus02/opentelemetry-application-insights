use crate::models::LimitedLenString8192;

#[derive(Debug)]
pub(crate) enum SeverityLevel {
    Verbose,
    Debug,
    Information,
    Warning,
    Error,
    None,
}

impl SeverityLevel {
    pub(crate) fn to_application_insights(&self) -> Option<i32> {
        match self {
            SeverityLevel::Verbose => Some(0),
            SeverityLevel::Debug => Some(1),
            SeverityLevel::Information => Some(1),
            SeverityLevel::Warning => Some(2),
            SeverityLevel::Error => Some(3),
            _ => None,
        }
    }
}

/// This will read from the tracing property 'level' and convert to our type.
impl From<Option<LimitedLenString8192>> for SeverityLevel {
    fn from(tracing_log_level: Option<LimitedLenString8192>) -> Self {
        match tracing_log_level {
            Some(log_level) => match log_level.as_ref() {
                "VERBOSE" => Self::Verbose,
                "DEBUG" => Self::Debug,
                "INFO" => Self::Information,
                "WARN" => Self::Warning,
                "ERROR" => Self::Error,
                _ => Self::None,
            },
            None => Self::None,
        }
    }
}
