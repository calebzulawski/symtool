use crate::elf::ElfTransform;
use crate::error::{Error, Result};
use crate::mach::MachTransform;
use crate::patch::Patch;
use goblin::Object;
use std::io::{Read, Seek, Write};

pub struct ObjectTransform {
    elf: Option<ElfTransform>,
    mach: Option<MachTransform>,
}

impl ObjectTransform {
    pub fn new() -> Self {
        Self {
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

    pub fn apply<R: Read + Seek, W: Write>(
        &mut self,
        reader: &mut R,
        writer: &mut W,
    ) -> Result<()> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        let patches = match Object::parse(&buf)? {
            Object::Elf(elf) => self.transform_elf(&buf, elf),
            Object::Mach(mach) => self.transform_mach(&buf, mach),
            _ => Err(Error::UnknownObject),
        }?;
        for patch in patches {
            patch.apply(&mut buf);
        }
        writer.write_all(&buf)?;
        Ok(())
    }

    fn transform_elf(&mut self, data: &[u8], elf: goblin::elf::Elf) -> Result<Vec<Patch>> {
        if let Some(transform) = &mut self.elf {
            transform.apply(&data, &elf)
        } else {
            Ok(Vec::new())
        }
    }

    fn transform_mach(&mut self, data: &[u8], mach: goblin::mach::Mach) -> Result<Vec<Patch>> {
        match mach {
            goblin::mach::Mach::Binary(macho) => self.transform_macho(data, macho),
            _ => Err(Error::FatBinaryUnsupported),
        }
    }

    fn transform_macho(&mut self, data: &[u8], macho: goblin::mach::MachO) -> Result<Vec<Patch>> {
        if let Some(transform) = &mut self.mach {
            transform.apply(&data, &macho)
        } else {
            Ok(Vec::new())
        }
    }
}
