use crate::elf::ElfTransform;
use crate::error::{Error, Result};
use crate::mach::MachTransform;
use crate::patch::Patch;
use goblin::Object;
use std::io::{Read, Seek, SeekFrom, Write};

fn get_variant_and_identifiers<R: Read + Seek>(
    reader: &mut R,
) -> Result<(ar::Variant, Vec<Vec<u8>>)> {
    let mut ar = ar::Archive::new(reader);
    let mut identifiers = Vec::new();
    loop {
        if let Some(entry) = ar.next_entry() {
            identifiers.push(entry?.header().identifier().to_vec());
        } else {
            break;
        }
    }
    let variant = ar.variant();
    ar.into_inner()?.seek(SeekFrom::Start(0))?;
    Ok((variant, identifiers))
}

enum ArchiveBuilder<'a> {
    Bsd(ar::Builder<&'a mut dyn Write>),
    Gnu(ar::GnuBuilder<&'a mut dyn Write>),
}

impl<'a> ArchiveBuilder<'a> {
    pub fn new<W: Write>(
        writer: &'a mut W,
        variant: ar::Variant,
        identifiers: Vec<Vec<u8>>,
    ) -> Self {
        if variant == ar::Variant::GNU {
            Self::Gnu(ar::GnuBuilder::new(writer, identifiers))
        } else {
            Self::Bsd(ar::Builder::new(writer))
        }
    }

    pub fn append<R: Read>(&mut self, header: &ar::Header, data: R) -> Result<()> {
        match self {
            Self::Bsd(ar) => ar.append(header, data)?,
            Self::Gnu(ar) => ar.append(header, data)?,
        }
        Ok(())
    }
}

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
        match goblin::peek(reader)? {
            goblin::Hint::Archive => self.apply_archive(reader, writer),
            _ => self.apply_object(reader, writer),
        }
    }

    fn apply_archive<R: Read + Seek, W: Write>(
        &mut self,
        reader: &mut R,
        writer: &mut W,
    ) -> Result<()> {
        let (variant, identifiers) = get_variant_and_identifiers(reader)?;
        let mut input = ar::Archive::new(reader);
        let mut output = ArchiveBuilder::new(writer, variant, identifiers);
        loop {
            if let Some(mut entry) = input.next_entry().transpose()? {
                let mut data = Vec::new();
                self.apply_object(&mut entry, &mut data)?;
                output.append(entry.header(), data.as_slice())?;
            } else {
                break;
            }
        }
        Ok(())
    }

    fn apply_object<R: Read + Seek, W: Write>(
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
