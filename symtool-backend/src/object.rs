//! Apply transformations to an object.

use crate::error::{Error, TransformError, TransformResult};
use crate::patch::Patch;
use goblin::elf::Elf;
use goblin::mach::MachO;
use std::convert::TryInto;

/// A generic object type
pub enum Object<'a> {
    Elf(Box<Elf<'a>>),
    MachO(Box<MachO<'a>>),
}

/// The type of a transformation applied to an object.
///
/// A transformation is expected to return a set of patches which are applied in order to the
/// binary.
pub type ObjectTransform<Error> =
    dyn for<'a> Fn(&'a [u8], Object) -> std::result::Result<Vec<Patch>, Error>;

/// Apply a transformation to a binary or an archive of binaries.
///
/// Objects are parsed from `reader` and stored into `writer`.
/// This function supports both BSD and GNU style archives.
pub fn transform_object<E>(
    object: &mut [u8],
    transformation: &ObjectTransform<E>,
) -> TransformResult<(), E>
where
    E: std::error::Error,
{
    // Determine the location of the object(s) to manipulate
    let mut objects = Vec::new();
    if let Ok(archive) = goblin::archive::Archive::parse(object) {
        for i in 0..archive.len() {
            let member = archive.get_at(i).unwrap();
            objects.push((
                member.offset.try_into().expect("object too large to parse"),
                member.header.size,
            ));
        }
    } else {
        objects.push((0, object.len()));
    }

    // Transform each object
    for (offset, size) in objects {
        let buf = &mut object[offset..offset + size];
        let object = match goblin::Object::parse(&buf)? {
            goblin::Object::Elf(elf) => Ok(Object::Elf(Box::new(elf))),
            goblin::Object::Mach(goblin::mach::Mach::Binary(macho)) => {
                Ok(Object::MachO(Box::new(macho)))
            }
            _ => Err(Error::UnknownObject),
        }?;
        let patches = transformation(&buf, object).map_err(TransformError::Transform)?;
        for patch in patches {
            patch.apply(buf);
        }
    }
    Ok(())
}
