use clap::{App, Arg, ArgMatches, SubCommand};
use goblin::elf::sym::{Sym, STV_DEFAULT, STV_HIDDEN};
use goblin::mach::symbols::{Nlist, N_PEXT, N_STAB};
use regex::RegexSet;

pub fn subcommand() -> App<'static, 'static> {
    SubCommand::with_name("visibility")
        .arg(
            Arg::with_name("hidden")
                .long("hidden")
                .takes_value(true)
                .value_name("PATTERN")
                .help("Sets all symbols with names matching regex PATTERN to hidden visibility")
                .required_unless("default"),
        )
        .arg(
            Arg::with_name("default")
                .long("default")
                .takes_value(true)
                .value_name("PATTERN")
                .help("Sets all symbols with names matching regex PATTERN to default visibility")
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

fn make_sym_hidden(s: &Sym, name: &str, verbosity: u64) -> Sym {
    if verbosity > 0 {
        println!("Set visibility hidden: {}", name);
    }
    Sym {
        st_other: (s.st_other & 0xfc) | STV_HIDDEN,
        ..s.clone()
    }
}

fn make_sym_default(s: &Sym, name: &str, verbosity: u64) -> Sym {
    if verbosity > 0 {
        println!("Set visibility default: {}", name);
    }
    Sym {
        st_other: (s.st_other & 0xfc) | STV_DEFAULT,
        ..s.clone()
    }
}

fn make_nlist_hidden(s: &Nlist, name: &str, verbosity: u64) -> Option<Nlist> {
    if s.n_type & N_STAB != 0u8 {
        None
    } else {
        if verbosity > 0 {
            println!("Set visibility hidden: {}", name);
        }
        Some(Nlist {
            n_type: s.n_type | N_PEXT,
            ..s.clone()
        })
    }
}

fn make_nlist_default(s: &Nlist, name: &str, verbosity: u64) -> Option<Nlist> {
    if s.n_type & N_STAB != 0u8 {
        None
    } else {
        if verbosity > 0 {
            println!("Set visibility default: {}", name);
        }
        Some(Nlist {
            n_type: s.n_type & !N_PEXT,
            ..s.clone()
        })
    }
}

pub fn run(matches: &ArgMatches, verbosity: u64) -> Result<(), Box<dyn std::error::Error>> {
    let hidden_regex = matches
        .values_of("hidden")
        .map(|regexes| RegexSet::new(regexes))
        .transpose()?;
    let default_regex = matches
        .values_of("default")
        .map(|regexes| RegexSet::new(regexes))
        .transpose()?;

    let transform: Box<objedit::object::ObjectTransform<crate::error::Error>> =
        Box::new(move |bytes, object| {
            let mut patches = Vec::new();
            match object {
                objedit::object::Object::Elf(elf) => {
                    if let Some(iter) = objedit::elf::SymtabIter::symtab_from_elf(bytes, &elf)? {
                        for (ref name, ref sym) in
                            iter.collect::<objedit::error::Result<Vec<_>>>()?
                        {
                            let debug_name = name.as_ref().map_or("unnamed symbol", |x| &x);
                            let new_sym = if let Some(name) = name {
                                if default_regex.is_some()
                                    && default_regex.as_ref().unwrap().is_match(name)
                                {
                                    Some(make_sym_default(sym, debug_name, verbosity))
                                } else if hidden_regex.is_some()
                                    && hidden_regex.as_ref().unwrap().is_match(name)
                                {
                                    Some(make_sym_hidden(sym, debug_name, verbosity))
                                } else {
                                    None
                                }
                            } else {
                                None
                            };
                            if new_sym.is_some() {
                                patches.push(sym.patch_with(new_sym.unwrap())?);
                            }
                        }
                    }
                }
                objedit::object::Object::MachO(mach) => {
                    if let Some(iter) = objedit::mach::SymtabIter::from_mach(bytes, &mach) {
                        for (ref name, ref nlist) in
                            iter.collect::<objedit::error::Result<Vec<_>>>()?
                        {
                            let debug_name = name.as_ref().map_or("unnamed symbol", |x| &x);
                            let new_nlist = if let Some(name) = name {
                                if default_regex.is_some()
                                    && default_regex.as_ref().unwrap().is_match(name)
                                {
                                    make_nlist_default(nlist, debug_name, verbosity)
                                } else if hidden_regex.is_some()
                                    && hidden_regex.as_ref().unwrap().is_match(name)
                                {
                                    make_nlist_hidden(nlist, debug_name, verbosity)
                                } else {
                                    None
                                }
                            } else {
                                None
                            };
                            if new_nlist.is_some() {
                                patches.push(nlist.patch_with(new_nlist.unwrap())?);
                            }
                        }
                    }
                }
            }
            Ok(patches)
        });

    let mut input = std::fs::File::open(matches.value_of("INPUT").unwrap())?;
    let mut output = std::fs::File::create(matches.value_of("OUTPUT").unwrap())?;
    objedit::object::transform_object(&mut input, &mut output, &transform)?;
    Ok(())
}
