#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    ObjEdit(objedit::error::Error),
    Regex(regex::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{}", e),
            Self::ObjEdit(e) => write!(f, "{}", e),
            Self::Regex(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::ObjEdit(e) => Some(e),
            Self::Regex(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<objedit::error::Error> for Error {
    fn from(err: objedit::error::Error) -> Self {
        Self::ObjEdit(err)
    }
}

impl From<regex::Error> for Error {
    fn from(err: regex::Error) -> Self {
        Self::Regex(err)
    }
}
