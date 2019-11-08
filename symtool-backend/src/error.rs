//! Errors returned by this crate.

pub type Result<T> = std::result::Result<T, Error>;
pub type TransformResult<T, E> = std::result::Result<T, TransformError<E>>;

/// An error
#[derive(Debug)]
pub enum Error {
    /// An I/O error
    Io(std::io::Error),

    /// An error from goblin, the binary parser
    Goblin(goblin::error::Error),

    /// An error from scroll, used by the binary parser
    Scroll(scroll::Error),

    /// The loaded object is malformed
    Malformed(String),

    /// Replacing a string failed
    ReplaceString {
        original: String,
        replacement: String,
    },

    /// The loaded object could not be recognized
    UnknownObject,

    /// Returned when loading a macOS fat binary
    FatBinaryUnsupported,

    /// The ELF section header did not match a symbol table
    WrongSectionHeader(String),

    /// A patch was too big to insert into the binary
    PatchTooBig,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{}", e),
            Self::Goblin(e) => write!(f, "{}", e),
            Self::Scroll(e) => write!(f, "{}", e),
            Self::Malformed(s) => write!(f, "{}", s),
            Self::ReplaceString { original, replacement } => {write!(
                f,
                "Replacement string (\"{}\") must be the same size or smaller than the original (\"{}\")", replacement, original)},
            Self::UnknownObject => write!(f, "Unknown object type"),
            Self::FatBinaryUnsupported => write!(f, "Fat MachO binaries are not yet supported"),
            Self::WrongSectionHeader(s) => write!(f, "{}", s),
            Self::PatchTooBig => write!(f, "Patched data too big for original location"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Goblin(e) => Some(e),
            Self::Scroll(e) => Some(e),
            _ => None,
        }
    }
}

impl From<goblin::error::Error> for Error {
    fn from(err: goblin::error::Error) -> Self {
        Self::Goblin(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<scroll::Error> for Error {
    fn from(err: scroll::Error) -> Self {
        Self::Scroll(err)
    }
}

/// An error returned by the object transformer
#[derive(Debug)]
pub enum TransformError<T>
where
    T: std::error::Error,
{
    /// An error produced by symtool
    SymTool(Error),

    /// An error produced by the transformer
    Transform(T),
}

impl<T> std::fmt::Display for TransformError<T>
where
    T: std::error::Error,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::SymTool(e) => write!(f, "{}", e),
            Self::Transform(e) => write!(f, "{}", e),
        }
    }
}

impl<T> std::error::Error for TransformError<T>
where
    T: std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SymTool(e) => Some(e),
            Self::Transform(e) => e.source(),
        }
    }
}

impl<T> From<Error> for TransformError<T>
where
    T: std::error::Error,
{
    fn from(err: Error) -> Self {
        Self::SymTool(err)
    }
}

impl<T> From<goblin::error::Error> for TransformError<T>
where
    T: std::error::Error,
{
    fn from(err: goblin::error::Error) -> Self {
        Self::SymTool(Error::Goblin(err))
    }
}

impl<T> From<std::io::Error> for TransformError<T>
where
    T: std::error::Error,
{
    fn from(err: std::io::Error) -> Self {
        Self::SymTool(Error::Io(err))
    }
}

impl<T> From<scroll::Error> for TransformError<T>
where
    T: std::error::Error,
{
    fn from(err: scroll::Error) -> Self {
        Self::SymTool(Error::Scroll(err))
    }
}
