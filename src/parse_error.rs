use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    //#[error("Failed parsing UTF-8 string: {0}")]
    //StringUtf8Error(#[from] FromUtf8Error),
    #[error("Hit end of data")]
    EndOfData,
}