use crate::error::{Error, Result};
use goblin::container::Ctx;
use scroll::ctx::{SizeWith, TryIntoCtx};

#[derive(Debug)]
pub struct Location {
    pub(crate) offset: usize,
    pub(crate) size: usize,
    pub(crate) ctx: Ctx,
}

#[derive(Debug)]
pub struct Patch {
    offset: usize,
    data: Vec<u8>,
}

impl Patch {
    pub fn new<T>(location: &Location, data: T) -> Result<Self>
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

    pub fn from_str(location: &Location, patch: &str) -> Result<Self> {
        if patch.len() > location.size {
            return Err(Error::PatchTooBig);
        }
        Ok(Self {
            offset: location.offset,
            data: patch.bytes().collect(),
        })
    }

    pub fn apply(&self, data: &mut [u8]) {
        data[self.offset..(self.offset + self.data.len())].clone_from_slice(&self.data);
    }
}
