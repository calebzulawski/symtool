//! Describe patches to an object.

use crate::error::{Error, Result};
use goblin::container::Ctx;
use scroll::ctx::{SizeWith, TryIntoCtx};

/// The location of a set of bytes in an object.
#[derive(Debug)]
pub(crate) struct Location {
    /// The byte offset into the object
    pub offset: usize,

    /// The number of bytes
    pub size: usize,

    /// Contextual information containing endianness and object type
    pub ctx: Ctx,
}

/// A value rooted to a location in an object.
#[derive(Debug)]
pub struct Rooted<T> {
    pub value: T,
    location: Location,
}

impl<T> Rooted<T> {
    pub(crate) fn new(location: Location, value: T) -> Self {
        Self { value, location }
    }

    /// Construct a patch that replaces this rooted value.
    pub fn patch_with<U>(&self, value: U) -> Result<Patch>
    where
        U: TryIntoCtx<Ctx, [u8], Error = goblin::error::Error> + SizeWith<Ctx>,
    {
        Patch::from_ctx(&self.location, value)
    }

    /// Construct a patch that replaces this rooted value with specific bytes.
    pub fn patch_with_bytes(&self, value: &[u8]) -> Result<Patch> {
        Patch::from_bytes(&self.location, value)
    }
}

impl<T> std::ops::Deref for Rooted<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

/// Represents a patch to an object.
#[derive(Debug)]
pub struct Patch {
    offset: usize,
    data: Vec<u8>,
}

impl Patch {
    fn from_ctx<T>(location: &Location, data: T) -> Result<Self>
    where
        T: TryIntoCtx<Ctx, [u8], Error = goblin::error::Error> + SizeWith<Ctx>,
    {
        let size = T::size_with(&location.ctx);
        if size > location.size {
            return Err(Error::PatchTooBig);
        }
        let mut buf = vec![0u8; size];
        data.try_into_ctx(&mut buf, location.ctx)?;
        Ok(Self {
            offset: location.offset,
            data: buf,
        })
    }

    fn from_bytes(location: &Location, data: &[u8]) -> Result<Self> {
        if data.len() > location.size {
            return Err(Error::PatchTooBig);
        }
        Ok(Self {
            offset: location.offset,
            data: data.to_vec(),
        })
    }

    /// Apply the patch to the bytes of an object.
    pub fn apply(&self, data: &mut [u8]) {
        data[self.offset..(self.offset + self.data.len())].clone_from_slice(&self.data);
    }
}
