use crate::error::{Error, Result};
use crate::patch::Patch;
use goblin::container::{Container, Ctx, Endian};
use goblin::elf::section_header;
use goblin::elf::sym::Sym;
use goblin::elf::Elf;
use goblin::strtab::Strtab;
use scroll::ctx::TryFromCtx;
use scroll::Pread;

pub fn replace_in_strtab(
    data: &mut [u8],
    strtab: &Strtab,
    offset: usize,
    replacement: &str,
) -> Result<()> {
    crate::manip::replace_str(
        data,
        strtab.get(offset).ok_or(Error::StrtabAccess)??,
        replacement,
    )
}

fn context_from_elf(elf: &Elf) -> Ctx {
    let container = if elf.is_64 {
        Container::Big
    } else {
        Container::Little
    };
    let endian = if elf.little_endian {
        Endian::Little
    } else {
        Endian::Big
    };
    Ctx::new(container, endian)
}

pub trait SymTransform: FnMut(&str, &Sym) -> (Option<String>, Option<Sym>) {}
impl<T> SymTransform for T where T: FnMut(&str, &Sym) -> (Option<String>, Option<Sym>) {}

pub struct ElfTransform {
    symtab: Vec<Box<dyn SymTransform>>,
}

impl ElfTransform {
    pub fn new() -> Self {
        Self { symtab: Vec::new() }
    }

    pub fn with_symtab_transform(&mut self, transform: Box<dyn SymTransform>) -> &mut Self {
        self.symtab.push(transform);
        self
    }

    pub fn with_symtab_transforms(
        &mut self,
        mut transforms: Vec<Box<dyn SymTransform>>,
    ) -> &mut Self {
        self.symtab.append(&mut transforms);
        self
    }

    pub fn apply(&mut self, bytes: &[u8], elf: &Elf) -> Result<Vec<Patch>> {
        let mut patches = Vec::new();
        patches.extend(self.apply_symtab(bytes, elf)?);
        Ok(patches)
    }

    fn apply_symtab(&mut self, bytes: &[u8], elf: &Elf) -> Result<Vec<Patch>> {
        let ctx = context_from_elf(elf);
        let mut patches = Vec::new();
        for header in &elf.section_headers {
            if header.sh_type as u32 == section_header::SHT_SYMTAB {
                if (header.sh_entsize as usize) < Sym::size(ctx.container) {
                    return Err(Error::Malformed("sh_entsize too small".to_string()));
                };
                let count = if header.sh_entsize == 0 {
                    0
                } else {
                    header.sh_size / header.sh_entsize
                };
                for index in 0..count {
                    let sym_offset = (header.sh_offset + index * header.sh_entsize) as usize;
                    let (sym, _) = Sym::try_from_ctx(&bytes[sym_offset..], ctx)?;
                    let name_offset = sym.st_name;
                    let name = bytes.pread(name_offset)?;
                    for f in &mut self.symtab {
                        let (new_name, new_sym) = f(name, &sym);
                        if let Some(new_sym) = new_sym {
                            patches.push(Patch::new(sym_offset, new_sym, &ctx)?);
                        }
                        if let Some(new_name) = new_name {
                            patches.push(Patch::from_str(name_offset, &new_name));
                        }
                    }
                }
            }
        }
        Ok(patches)
    }
}
