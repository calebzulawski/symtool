use crate::error::{Error, Result};
use goblin::container::Ctx;
use scroll::ctx::{SizeWith, TryIntoCtx};

#[derive(Debug)]
pub(crate) struct Location {
    pub offset: usize,
    pub size: usize,
    pub ctx: Ctx,
}

#[derive(Debug)]
pub struct Rooted<T> {
    pub value: T,
    location: Location,
}

impl<T> Rooted<T> {
    pub(crate) fn new(location: Location, value: T) -> Self {
        Self { value, location }
    }

    pub fn patch_with<U>(&self, value: U) -> Result<Patch>
    where
        U: TryIntoCtx<Ctx, [u8], Error = goblin::error::Error, Size = usize>
            + SizeWith<Ctx, Units = usize>,
    {
        Patch::from_ctx(&self.location, value)
    }

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

#[derive(Debug)]
pub struct Patch {
    offset: usize,
    data: Vec<u8>,
}

impl Patch {
    fn from_ctx<T>(location: &Location, data: T) -> Result<Self>
    where
        T: TryIntoCtx<Ctx, [u8], Error = goblin::error::Error, Size = usize>
            + SizeWith<Ctx, Units = usize>,
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

    pub fn apply(&self, data: &mut [u8]) {
        data[self.offset..(self.offset + self.data.len())].clone_from_slice(&self.data);
    }
}
