use crate::error::Result;
use crate::patch::{Location, Rooted};
use goblin::container::{Container, Ctx, Endian};
use goblin::mach::load_command::{CommandVariant, SymtabCommand};
use goblin::mach::symbols::Nlist;
use goblin::mach::MachO;
use scroll::ctx::{SizeWith, TryFromCtx};
use scroll::Pread;

fn context_from_macho(macho: &MachO) -> Ctx {
    let container = if macho.is_64 {
        Container::Big
    } else {
        Container::Little
    };
    let endian = if macho.little_endian {
        Endian::Little
    } else {
        Endian::Big
    };
    Ctx::new(container, endian)
}

pub struct SymtabIter<'a> {
    bytes: &'a [u8],
    ctx: Ctx,
    symoff: usize,
    stroff: usize,
    count: usize,
    index: usize,
}

impl<'a> SymtabIter<'a> {
    pub fn from_load_command(bytes: &'a [u8], command: &SymtabCommand, ctx: Ctx) -> Self {
        Self {
            bytes: bytes,
            ctx: ctx,
            symoff: command.symoff as usize,
            stroff: command.stroff as usize,
            count: command.nsyms as usize,
            index: 0,
        }
    }

    pub fn from_mach(bytes: &'a [u8], mach: &MachO) -> Option<Self> {
        let ctx = context_from_macho(mach);
        for command in &mach.load_commands {
            if let CommandVariant::Symtab(command) = command.command {
                return Some(Self::from_load_command(bytes, &command, ctx));
            }
        }
        None
    }
}

impl<'a> std::iter::Iterator for SymtabIter<'a> {
    type Item = Result<(Option<Rooted<&'a str>>, Rooted<Nlist>)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            None
        } else {
            Some((|| {
                let nlist_offset = self.symoff + self.index * Nlist::size_with(&self.ctx);
                self.index += 1;
                let nlist = {
                    let (nlist, nlist_size) =
                        Nlist::try_from_ctx(&self.bytes[nlist_offset..], self.ctx)?;
                    let location = Location {
                        offset: nlist_offset,
                        size: nlist_size,
                        ctx: self.ctx,
                    };
                    Rooted::new(location, nlist)
                };
                let name = if nlist.n_strx != 0 {
                    let offset = self.stroff + nlist.n_strx as usize;
                    let name: &str = self.bytes.pread(offset)?;
                    let location = Location {
                        offset: offset,
                        size: name.len(),
                        ctx: self.ctx,
                    };
                    Some(Rooted::new(location, name))
                } else {
                    None
                };
                Ok((name, nlist))
            })())
        }
    }
}
