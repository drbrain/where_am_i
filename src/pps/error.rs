use std::fmt;

#[derive(Debug)]
pub enum Error {
    CannotCaptureAssert(String),
    CannotGetParameters(String),
    CannotSetParameters(String),
    CannotWait(String),
    CapabilitiesFailed(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::CannotCaptureAssert(n) => {
                write!(f, "cannot capture assert events for PPS device {}", n)
            }
            Error::CannotGetParameters(n) => {
                write!(f, "cannot get parameters for PPS device {}", n)
            }
            Error::CannotSetParameters(n) => {
                write!(f, "cannot set parameters for PPS device {}", n)
            }
            Error::CannotWait(n) => write!(f, "{} cannot wait for PPS events", n),
            Error::CapabilitiesFailed(n) => {
                write!(f, "unable to get capabilities of PPS device {}", n)
            }
        }
    }
}

impl std::error::Error for Error {}
