use crate::error::Result;
use crate::patch::{Location, Patch};
use goblin::container::{Container, Ctx, Endian};
use goblin::mach::load_command::CommandVariant;
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
        let ctx = context_from_macho(macho);
        let mut patches = Vec::new();
        for command in &macho.load_commands {
            if let CommandVariant::Symtab(symtab_command) = command.command {
                for index in 0..(symtab_command.nsyms as usize) {
                    let nlist_offset =
                        symtab_command.symoff as usize + index * Nlist::size_with(&ctx);
                    let (nlist, nlist_size) = Nlist::try_from_ctx(&bytes[nlist_offset..], ctx)?;
                    let nlist_location = Location {
                        offset: nlist_offset,
                        size: nlist_size,
                        ctx: ctx,
                    };
                    let name_offset = symtab_command.stroff as usize + nlist.n_strx as usize;
                    let name: &str = bytes.pread(name_offset)?;
                    let name_location = Location {
                        offset: name_offset,
                        size: name.len(),
                        ctx: ctx,
                    };
                    for f in &mut self.symtab {
                        let (new_name, new_nlist) = f(name, &nlist);
                        if let Some(new_nlist) = new_nlist {
                            patches.push(Patch::new(&nlist_location, new_nlist)?);
                        }
                        if let Some(new_name) = new_name {
                            patches.push(Patch::from_str(&name_location, &new_name)?);
                        }
                    }
                }
            }
        }
        Ok(patches)
    }
}
