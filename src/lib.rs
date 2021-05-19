pub mod sharedlock;
pub use sharedlock::SharedLock;

#[derive(Debug)]
pub enum Error {
    DeadLockError,
    Poisoned,
}

pub type Result<T> = std::result::Result<T, Error>;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DeadLockError => {
                write!(f, "DeadLock")
            }
            Error::Poisoned => {
                write!(f, "Poisoned")
            }
        }
    }
}
