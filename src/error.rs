use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Another instance of Stevedore is already running")]
    AlreadyRunning,
}
