use crate::error::Result;
use clap::{App, Arg, ArgMatches, SubCommand};
use goblin::elf::sym::{Sym, STV_DEFAULT, STV_HIDDEN};
use goblin::mach::symbols::{Nlist, N_PEXT, N_STAB};
use regex::RegexSet;

pub fn subcommand() -> App<'static, 'static> {
    SubCommand::with_name("visibility")
        .arg(
            Arg::with_name("all-hidden")
                .long("all-hidden")
                .takes_value(false)
                .help("Sets all symbols to hidden visibility")
                .conflicts_with("all-default")
                .conflicts_with("default")
                .conflicts_with("hidden")
                .required(true),
        )
        .arg(
            Arg::with_name("all-default")
                .long("all-default")
                .takes_value(false)
                .help("Sets all symbols to default visibility")
                .conflicts_with("all-hidden")
                .conflicts_with("default")
                .conflicts_with("hidden")
                .required(true),
        )
        .arg(
            Arg::with_name("hidden")
                .long("hidden")
                .takes_value(true)
                .value_name("PATTERN")
                .help("Sets all symbols with names matching regex PATTERN to hidden visibility")
                .conflicts_with("all-hidden")
                .conflicts_with("all-default")
                .required_unless("default"),
        )
        .arg(
            Arg::with_name("default")
                .long("default")
                .takes_value(true)
                .value_name("PATTERN")
                .help("Sets all symbols with names matching regex PATTERN to default visibility")
                .conflicts_with("all-hidden")
                .conflicts_with("all-default")
                .required_unless("hidden"),
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

enum Mode {
    AllHidden,
    AllDefault,
    Regex {
        hidden: Option<RegexSet>,
        default: Option<RegexSet>,
    },
}

pub fn run(matches: &ArgMatches, verbosity: u64) -> Result<()> {
    let mode = if matches.is_present("all-hidden") {
        Mode::AllHidden
    } else if matches.is_present("all-default") {
        Mode::AllDefault
    } else {
        let hidden = matches
            .values_of("hidden")
            .map(|regexes| RegexSet::new(regexes))
            .transpose()?;
        let default = matches
            .values_of("default")
            .map(|regexes| RegexSet::new(regexes))
            .transpose()?;
        Mode::Regex {
            hidden: hidden,
            default: default,
        }
    };

    let transform: Box<objedit::object::ObjectTransform> = Box::new(move |bytes, object| {
        let mut patches = Vec::new();
        match object {
            objedit::object::Object::Elf(elf) => {
                if let Some(iter) = objedit::elf::SymtabIter::symtab_from_elf(bytes, &elf).unwrap()
                {
                    for sym_info in iter {
                        let sym_info = sym_info.unwrap();
                        let sym = match mode {
                            Mode::AllHidden => Some(make_sym_hidden(&sym_info.sym)),
                            Mode::AllDefault => Some(make_sym_default(&sym_info.sym)),
                            Mode::Regex {
                                ref hidden,
                                ref default,
                            } => {
                                if default.is_some()
                                    && default.as_ref().unwrap().is_match(sym_info.name)
                                {
                                    Some(make_sym_default(&sym_info.sym))
                                } else if hidden.is_some()
                                    && hidden.as_ref().unwrap().is_match(sym_info.name)
                                {
                                    Some(make_sym_hidden(&sym_info.sym))
                                } else {
                                    None
                                }
                            }
                        };
                        if sym.is_some() {
                            patches.push(
                                objedit::patch::Patch::new(&sym_info.sym_location, sym.unwrap())
                                    .unwrap(),
                            );
                        }
                    }
                }
            }
            objedit::object::Object::MachO(mach) => {
                if let Some(iter) = objedit::mach::SymtabIter::from_mach(bytes, &mach) {
                    for nlist_info in iter {
                        let nlist_info = nlist_info.unwrap();
                        let nlist = match mode {
                            Mode::AllHidden => make_nlist_hidden(&nlist_info.nlist),
                            Mode::AllDefault => make_nlist_default(&nlist_info.nlist),
                            Mode::Regex {
                                ref hidden,
                                ref default,
                            } => {
                                if default.is_some()
                                    && default.as_ref().unwrap().is_match(nlist_info.name)
                                {
                                    make_nlist_default(&nlist_info.nlist)
                                } else if hidden.is_some()
                                    && hidden.as_ref().unwrap().is_match(nlist_info.name)
                                {
                                    make_nlist_hidden(&nlist_info.nlist)
                                } else {
                                    None
                                }
                            }
                        };
                        if nlist.is_some() {
                            patches.push(
                                objedit::patch::Patch::new(
                                    &nlist_info.nlist_location,
                                    nlist.unwrap(),
                                )
                                .unwrap(),
                            );
                        }
                    }
                }
            }
        }
        patches
    });

    let mut input = std::fs::File::open(matches.value_of("INPUT").unwrap())?;
    let mut output = std::fs::File::create(matches.value_of("OUTPUT").unwrap())?;
    objedit::object::transform_object(&mut input, &mut output, &transform)?;
    Ok(())
}
