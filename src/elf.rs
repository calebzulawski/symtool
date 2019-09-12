use crate::error::{Error, Result};
use goblin::container::{Container, Ctx, Endian};
use goblin::elf::section_header;
use goblin::elf::sym::Sym;
use goblin::elf::Elf;
use goblin::strtab::Strtab;
use scroll::ctx::{TryFromCtx, TryIntoCtx};

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

pub fn transform_symtab<F>(bytes: &mut [u8], elf: &Elf, mut f: F) -> Result<()>
where
    F: FnMut(&str, Sym) -> (Option<String>, Option<Sym>),
{
    let ctx = context_from_elf(elf);
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
            let begin = header.sh_offset as usize;
            let end = begin + (count * header.sh_entsize) as usize;
            let mut string_replacements = Vec::new();
            for sym_bytes in bytes[begin..end].chunks_exact_mut(header.sh_entsize as usize) {
                let (sym, _) = Sym::try_from_ctx(sym_bytes, ctx)?;
                let name = elf.strtab.get(sym.st_name).ok_or(Error::StrtabAccess)??;
                let name_index = sym.st_name;
                let (new_name, new_sym) = f(name, sym);
                if let Some(new_name) = new_name {
                    string_replacements.push((name_index, new_name));
                }
                if let Some(new_sym) = new_sym {
                    new_sym.try_into_ctx(sym_bytes, ctx);
                }
            }
            for (index, string) in string_replacements {
                replace_in_strtab(bytes, &elf.strtab, index, &string);
            }
        }
    }
    Ok(())
}
