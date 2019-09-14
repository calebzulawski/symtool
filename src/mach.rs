use crate::error::Result;
use crate::patch::{Location, Patch};
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

pub trait NlistTransform: FnMut(&str, &Nlist) -> (Option<String>, Option<Nlist>) {}
impl<T> NlistTransform for T where T: FnMut(&str, &Nlist) -> (Option<String>, Option<Nlist>) {}

pub struct MachTransform {
    symtab: Vec<Box<dyn NlistTransform>>,
}

impl MachTransform {
    pub fn new() -> Self {
        Self { symtab: Vec::new() }
    }

    pub fn with_symtab_transform(&mut self, transform: Box<dyn NlistTransform>) -> &mut Self {
        self.symtab.push(transform);
        self
    }

    pub fn with_symtab_transforms(
        &mut self,
        mut transforms: Vec<Box<dyn NlistTransform>>,
    ) -> &mut Self {
        self.symtab.append(&mut transforms);
        self
    }

    pub fn apply(&mut self, bytes: &[u8], mach: &MachO) -> Result<Vec<Patch>> {
        let mut patches = Vec::new();
        patches.extend(self.apply_symtab(bytes, mach)?);
        Ok(patches)
    }

    fn apply_symtab(&mut self, bytes: &[u8], macho: &MachO) -> Result<Vec<Patch>> {
        if self.symtab.is_empty() {
            return Ok(Vec::new());
        }
        let mut patches = Vec::new();
        if let Some(iter) = SymtabIter::from_mach(bytes, macho) {
            for nlist_info in iter {
                let nlist_info = nlist_info?;
                for f in &mut self.symtab {
                    let (new_name, new_nlist) = f(nlist_info.name, &nlist_info.nlist);
                    if let Some(new_nlist) = new_nlist {
                        patches.push(Patch::new(&nlist_info.nlist_location, new_nlist)?);
                    }
                    if let Some(new_name) = new_name {
                        patches.push(Patch::from_str(&nlist_info.name_location, &new_name)?);
                    }
                }
            }
        }
        Ok(patches)
    }
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
    fn from_load_command(bytes: &'a [u8], command: &SymtabCommand, ctx: Ctx) -> Self {
        Self {
            bytes: bytes,
            ctx: ctx,
            symoff: command.symoff as usize,
            stroff: command.stroff as usize,
            count: command.nsyms as usize,
            index: 0,
        }
    }

    fn from_mach(bytes: &'a [u8], mach: &MachO) -> Option<Self> {
        let ctx = context_from_macho(mach);
        for command in &mach.load_commands {
            if let CommandVariant::Symtab(command) = command.command {
                return Some(Self::from_load_command(bytes, &command, ctx));
            }
        }
        None
    }
}

pub struct NlistInfo<'a> {
    name: &'a str,
    name_location: Location,
    nlist: Nlist,
    nlist_location: Location,
}

impl<'a> std::iter::Iterator for SymtabIter<'a> {
    type Item = Result<NlistInfo<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            None
        } else {
            Some((|| {
                let nlist_offset = self.symoff + self.index * Nlist::size_with(&self.ctx);
                self.index += 1;
                let (nlist, nlist_size) =
                    Nlist::try_from_ctx(&self.bytes[nlist_offset..], self.ctx)?;
                let nlist_location = Location {
                    offset: nlist_offset,
                    size: nlist_size,
                    ctx: self.ctx,
                };
                let name_offset = self.stroff + nlist.n_strx as usize;
                let name: &str = self.bytes.pread(name_offset)?;
                let name_location = Location {
                    offset: name_offset,
                    size: name.len(),
                    ctx: self.ctx,
                };
                Ok(NlistInfo {
                    name: name,
                    name_location: name_location,
                    nlist: nlist,
                    nlist_location: nlist_location,
                })
            })())
        }
    }
}
