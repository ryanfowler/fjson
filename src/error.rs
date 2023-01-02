#[derive(Copy, Clone, Debug)]
pub enum Error {
    BadWrite,
    UnexpectedCharacter((usize, char)),
    UnexpectedToken(&'static str),
    UnexpectedEOF,
}

impl From<std::fmt::Error> for Error {
    fn from(_value: std::fmt::Error) -> Self {
        Error::BadWrite
    }
}
