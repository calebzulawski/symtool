use crate::elf::SymMap;
use crate::mach::NlistMap;
use goblin::Object;
use std::io::{Read, Seek, Write};

pub struct ObjectTransform {
    elf: Option<ElfTransform>,
    mach: Option<MachTransform>,
}

impl ObjectTransform {
    pub fn new() -> Self {
        ObjectTransformation {
            elf: None,
            mach: None,
        }
    }

    pub fn with_elf_transform(&mut self, transform: ElfTransform) -> &mut Self {
        self.elf = Some(transform);
        self
    }

    pub fn with_mach_transform(&mut self, transform: MachTransform) -> &mut Self {
        self.mach = Some(transform);
        self
    }

    pub fn apply<R: Read + Seek, W: Write>(reader: &R, writer: &W) -> Result<()> {
        let mut buf = Vec::new();
        reader.read_to_end(&buf);
        let patches = match Object::parse(&buf) {
            Elf(elf) => self.transform_elf(&buf, elf),
            Mach(mach) => self.transform_mach(&buf, mach),
            _ => Err(Error::UnknownObject),
        }?;
    }

    fn transform_elf(data: &[u8], elf: goblin::Elf) -> Result<Vec<Patch>> {
        if let Some(transform) = &self.elf {
            transform.apply(&data, &elf)
        } else {
            Ok(Vec::new())
        }
    }

    fn transform_mach(data: &[u8], mach: goblin::Mach) -> Result<Vec<Patch>> {
        if let Some(transform) = &self.mach {
            transform.apply(&data, &mach)
        } else {
            Ok(Vec::new())
        }
    }
}
