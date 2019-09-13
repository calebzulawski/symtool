use crate::error::Result;
use clap::{App, Arg, ArgMatches, SubCommand};
use goblin::elf::sym::{Sym, STV_DEFAULT, STV_HIDDEN};
use goblin::mach::symbols::{Nlist, N_PEXT, N_STAB};
use objedit::elf::ElfTransform;
use objedit::mach::MachTransform;
use objedit::ObjectTransform;
use regex::RegexSet;
use std::rc::Rc;

pub fn subcommand() -> App<'static, 'static> {
    SubCommand::with_name("visibility")
        .arg(
            Arg::with_name("all-hidden")
                .long("all-hidden")
                .takes_value(false)
                .help("Sets all symbols to hidden visibility")
                .conflicts_with("all-default")
                .conflicts_with("default")
                .conflicts_with("hidden"),
        )
        .arg(
            Arg::with_name("all-default")
                .long("all-default")
                .takes_value(false)
                .help("Sets all symbols to default visibility")
                .conflicts_with("all-hidden")
                .conflicts_with("default")
                .conflicts_with("hidden"),
        )
        .arg(
            Arg::with_name("hidden")
                .long("hidden")
                .takes_value(true)
                .value_name("PATTERN")
                .help("Sets all symbols with names matching regex PATTERN to hidden visibility"),
        )
        .arg(
            Arg::with_name("default")
                .long("default")
                .takes_value(true)
                .value_name("PATTERN")
                .help("Sets all symbols with names matching regex PATTERN to default visibility"),
        )
        .arg(
            Arg::with_name("INPUT")
                .help("Path to source object file")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("OUTPUT")
                .help("Path to modified object file")
                .required(true)
                .index(2),
        )
}

fn make_sym_hidden(s: &Sym) -> Sym {
    Sym {
        st_other: (s.st_other & 0xfc) | STV_HIDDEN,
        ..s.clone()
    }
}

fn make_sym_default(s: &Sym) -> Sym {
    Sym {
        st_other: (s.st_other & 0xfc) | STV_DEFAULT,
        ..s.clone()
    }
}

fn make_nlist_hidden(s: &Nlist) -> Option<Nlist> {
    if s.n_type & N_STAB != 0u8 {
        None
    } else {
        Some(Nlist {
            n_type: s.n_type | N_PEXT,
            ..s.clone()
        })
    }
}

fn make_nlist_default(s: &Nlist) -> Option<Nlist> {
    if s.n_type & N_STAB != 0u8 {
        None
    } else {
        Some(Nlist {
            n_type: s.n_type & !N_PEXT,
            ..s.clone()
        })
    }
}

pub fn run(matches: &ArgMatches, verbosity: u64) -> Result<()> {
    let mut elf = ElfTransform::new();
    let mut mach = MachTransform::new();
    if matches.is_present("all-hidden") {
        elf.with_symtab_transform(Box::new(|_, sym| (None, Some(make_sym_hidden(sym)))));
        mach.with_symtab_transform(Box::new(|_, nlist| (None, make_nlist_hidden(nlist))));
    }
    if matches.is_present("all-default") {
        elf.with_symtab_transform(Box::new(|_, sym| (None, Some(make_sym_default(sym)))));
        mach.with_symtab_transform(Box::new(|_, nlist| (None, make_nlist_default(nlist))));
    }
    if let Some(regexes) = matches.values_of("hidden") {
        let regex = Rc::new(RegexSet::new(regexes)?);
        let regex2 = regex.clone();
        elf.with_symtab_transform(Box::new(move |name, sym| {
            let new_sym = if regex.is_match(name) {
                Some(make_sym_default(sym))
            } else {
                None
            };
            (None, new_sym)
        }));
        mach.with_symtab_transform(Box::new(move |name, nlist| {
            let new_nlist = if regex2.is_match(name) {
                make_nlist_default(nlist)
            } else {
                None
            };
            (None, new_nlist)
        }));
    }
    if let Some(regexes) = matches.values_of("default") {
        let regex = Rc::new(RegexSet::new(regexes)?);
        let regex2 = regex.clone();
        elf.with_symtab_transform(Box::new(move |name, sym| {
            let new_sym = if regex.is_match(name) {
                Some(make_sym_hidden(sym))
            } else {
                None
            };
            (None, new_sym)
        }));
        mach.with_symtab_transform(Box::new(move |name, nlist| {
            let new_nlist = if regex2.is_match(name) {
                make_nlist_hidden(nlist)
            } else {
                None
            };
            (None, new_nlist)
        }));
    }
    let mut input = std::fs::File::open(matches.value_of("INPUT").unwrap())?;
    let mut output = std::fs::File::create(matches.value_of("OUTPUT").unwrap())?;
    ObjectTransform::new()
        .with_elf_transform(elf)
        .with_mach_transform(mach)
        .apply(&mut input, &mut output)?;
    Ok(())
}
