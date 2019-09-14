use crate::error::{Error, Result};
use crate::patch::Location;
use goblin::container::{Container, Ctx, Endian};
use goblin::elf::section_header::{SHT_DYNSYM, SHT_SYMTAB};
use goblin::elf::sym::Sym;
use goblin::elf::{Elf, SectionHeader};
use scroll::ctx::TryFromCtx;
use scroll::Pread;

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

pub struct SymtabIter<'a> {
    bytes: &'a [u8],
    ctx: Ctx,
    offset: usize,
    step: usize,
    count: usize,
    index: usize,
}

impl<'a> SymtabIter<'a> {
    pub fn from_section_header(bytes: &'a [u8], header: &SectionHeader, ctx: Ctx) -> Result<Self> {
        if header.sh_type != SHT_SYMTAB && header.sh_type != SHT_DYNSYM {
            return Err(Error::WrongSectionHeader(
                "symtab requires sh_type equal to SHT_SYMTAB or SHT_DYNSYM".to_string(),
            ));
        }
        if (header.sh_entsize as usize) < Sym::size(ctx.container) {
            return Err(Error::Malformed("sh_entsize too small".to_string()));
        };
        Ok(Self {
            bytes: bytes,
            ctx: ctx,
            offset: header.sh_offset as usize,
            step: header.sh_entsize as usize,
            count: if header.sh_entsize == 0 {
                0
            } else {
                header.sh_size / header.sh_entsize
            } as usize,
            index: 0,
        })
    }

    pub fn symtab_from_elf(bytes: &'a [u8], elf: &Elf) -> Result<Option<Self>> {
        let ctx = context_from_elf(elf);
        for header in &elf.section_headers {
            if header.sh_type == SHT_SYMTAB {
                return Some(Self::from_section_header(bytes, header, ctx)).transpose();
            }
        }
        Ok(None)
    }

    pub fn dynsym_from_elf(bytes: &'a [u8], elf: &Elf) -> Result<Option<Self>> {
        let ctx = context_from_elf(elf);
        for header in &elf.section_headers {
            if header.sh_type == SHT_DYNSYM {
                return Some(Self::from_section_header(bytes, header, ctx)).transpose();
            }
        }
        Ok(None)
    }
}

pub struct SymInfo<'a> {
    pub name: &'a str,
    pub name_location: Location,
    pub sym: Sym,
    pub sym_location: Location,
}

impl<'a> std::iter::Iterator for SymtabIter<'a> {
    type Item = Result<SymInfo<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            None
        } else {
            Some((|| {
                let sym_offset = self.offset + self.index * self.step;
                self.index += 1;
                let (sym, sym_size) = Sym::try_from_ctx(&self.bytes[sym_offset..], self.ctx)?;
                let sym_location = Location {
                    offset: sym_offset,
                    size: sym_size,
                    ctx: self.ctx,
                };
                let name_offset = sym.st_name;
                let name: &str = self.bytes.pread(name_offset)?;
                let name_location = Location {
                    offset: name_offset,
                    size: name.len(),
                    ctx: self.ctx,
                };
                Ok(SymInfo {
                    name: name,
                    name_location: name_location,
                    sym: sym,
                    sym_location: sym_location,
                })
            })())
        }
    }
}
