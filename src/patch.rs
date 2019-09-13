use crate::error::Result;
use goblin::container::Ctx;
use scroll::ctx::{SizeWith, TryIntoCtx};

#[derive(Debug)]
pub struct Patch {
    offset: usize,
    data: Vec<u8>,
}

impl Patch {
    pub fn new<T>(offset: usize, patch: T, ctx: &Ctx) -> Result<Self>
    where
        T: TryIntoCtx<Ctx, [u8], Error = goblin::error::Error, Size = usize>
            + SizeWith<Ctx, Units = usize>,
    {
        let mut data = vec![0u8; T::size_with(&ctx)];
        patch.try_into_ctx(&mut data, *ctx)?;
        Ok(Self {
            offset: offset,
            data: data,
        })
    }

    pub fn from_str(offset: usize, patch: &str) -> Self {
        Self {
            offset: offset,
            data: patch.bytes().collect(),
        }
    }

    pub fn apply(&self, data: &mut [u8]) {
        data[self.offset..(self.offset + self.data.len())].clone_from_slice(&self.data);
    }
}
