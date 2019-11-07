use crate::error::{Error, Result, TransformError, TransformResult};
use crate::patch::Patch;
use goblin::elf::Elf;
use goblin::mach::MachO;
use std::io::{Read, Seek, SeekFrom, Write};

fn get_variant_and_identifiers<R: Read + Seek>(
    reader: &mut R,
) -> Result<(ar::Variant, Vec<Vec<u8>>)> {
    let mut ar = ar::Archive::new(reader);
    let mut identifiers = Vec::new();
    while let Some(entry) = ar.next_entry() {
        identifiers.push(entry?.header().identifier().to_vec());
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

pub enum Object<'a> {
    Elf(Box<Elf<'a>>),
    MachO(Box<MachO<'a>>),
}

pub type ObjectTransform<Error> =
    dyn for<'a> Fn(&'a [u8], Object) -> std::result::Result<Vec<Patch>, Error>;

pub fn transform_object<'b, R, W, E>(
    reader: &mut R,
    writer: &mut W,
    transformation: &'b ObjectTransform<E>,
) -> TransformResult<(), E>
where
    R: Read + Seek,
    W: Write,
    E: std::error::Error,
{
    match goblin::peek(reader)? {
        goblin::Hint::Archive => transform_archive(reader, writer, transformation),
        _ => transform_single(reader, writer, transformation),
    }
}

fn transform_archive<'b, R, W, E>(
    reader: &mut R,
    writer: &mut W,
    transformation: &'b ObjectTransform<E>,
) -> TransformResult<(), E>
where
    R: Read + Seek,
    W: Write,
    E: std::error::Error,
{
    let (variant, identifiers) = get_variant_and_identifiers(reader)?;
    let mut input = ar::Archive::new(reader);
    let mut output = ArchiveBuilder::new(writer, variant, identifiers);
    while let Some(mut entry) = input.next_entry().transpose()? {
        let mut data = Vec::new();
        transform_single(&mut entry, &mut data, transformation)?;
        output.append(entry.header(), data.as_slice())?;
    }
    Ok(())
}

fn transform_single<'b, R, W, E>(
    reader: &mut R,
    writer: &mut W,
    transformation: &'b ObjectTransform<E>,
) -> TransformResult<(), E>
where
    R: Read + Seek,
    W: Write,
    E: std::error::Error,
{
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    let object = match goblin::Object::parse(&buf)? {
        goblin::Object::Elf(elf) => Ok(Object::Elf(Box::new(elf))),
        goblin::Object::Mach(goblin::mach::Mach::Binary(macho)) => {
            Ok(Object::MachO(Box::new(macho)))
        }
        _ => Err(Error::UnknownObject),
    }?;
    let patches = transformation(&buf, object).map_err(TransformError::Transform)?;
    for patch in patches {
        patch.apply(&mut buf);
    }
    writer.write_all(&buf)?;
    Ok(())
}
