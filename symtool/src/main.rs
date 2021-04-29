use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, Arg, ArgMatches,
};
use goblin::elf::sym::{Sym, STB_GLOBAL, STB_WEAK, STT_NOTYPE, STV_DEFAULT, STV_HIDDEN};
use goblin::mach::symbols::{Nlist, N_PEXT, N_STAB};
use regex::RegexSet;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::ops::Deref;

use symtool_backend as backend;

mod error;
use crate::error::Error;

fn main() {
    let matches = app_from_crate!()
        .arg(
            Arg::with_name("verbose")
                .long("verbose")
                .short("v")
                .help("Print information for each operation performed"),
        )
        .arg(
            Arg::with_name("rename")
                .long("rename")
                .number_of_values(2)
                .multiple(true)
                .value_names(&["OLD-NAME", "NEW-NAME"])
                .help("Renames symbols named OLD-NAME to NEW-NAME")
                .long_help("Renames symbols named OLD-NAME to NEW-NAME. Since string tables are simply patched and not rewritten, NEW-NAME must not have more characters than OLD-NAME")
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
                .help("Sets all symbols with names matching regex PATTERN to default visibility")
                .long_help(
                    "Sets all symbols with names matching regex PATTERN to default visibility.  --default takes precedance over --hidden when both patterns match a symbol name.",
                ),
        )
        .arg(
            Arg::with_name("INPUT")
                .help("Path to source object or archive file")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("OUTPUT")
                .help("Path to output file")
                .required(true)
                .index(2),
        )
        .get_matches();

    run(&matches).unwrap_or_else(|e| {
        eprintln!("error: {}", e);
        std::process::exit(-1)
    });
}

fn make_sym_hidden(s: &Sym, name: &str, verbose: bool) -> Sym {
    if verbose {
        println!("Set visibility hidden: {}", name);
    }
    Sym {
        st_other: (s.st_other & 0xfc) | STV_HIDDEN,
        ..*s
    }
}

fn make_sym_default(s: &Sym, name: &str, verbose: bool) -> Sym {
    if verbose {
        println!("Set visibility default: {}", name);
    }
    Sym {
        st_other: (s.st_other & 0xfc) | STV_DEFAULT,
        ..*s
    }
}

fn change_sym_vis(
    sym: &Sym,
    name: &str,
    verbose: bool,
    hidden_regex: &Option<RegexSet>,
    default_regex: &Option<RegexSet>,
) -> Option<Sym> {
    if (sym.st_bind() != STB_GLOBAL && sym.st_bind() != STB_WEAK) || sym.st_type() == STT_NOTYPE {
        return None;
    }
    if default_regex.is_some() && default_regex.as_ref().unwrap().is_match(name) {
        Some(make_sym_default(sym, name, verbose))
    } else if hidden_regex.is_some() && hidden_regex.as_ref().unwrap().is_match(name) {
        Some(make_sym_hidden(sym, name, verbose))
    } else {
        None
    }
}

fn make_nlist_hidden(s: &Nlist, name: &str, verbose: bool) -> Option<Nlist> {
    if s.n_type & N_STAB != 0u8 {
        None
    } else {
        if verbose {
            println!("Set visibility hidden: {}", name);
        }
        Some(Nlist {
            n_type: s.n_type | N_PEXT,
            ..s.clone()
        })
    }
}

fn make_nlist_default(s: &Nlist, name: &str, verbose: bool) -> Option<Nlist> {
    if s.n_type & N_STAB != 0u8 {
        None
    } else {
        if verbose {
            println!("Set visibility default: {}", name);
        }
        Some(Nlist {
            n_type: s.n_type & !N_PEXT,
            ..s.clone()
        })
    }
}

fn change_nlist_vis(
    nlist: &Nlist,
    name: &str,
    verbose: bool,
    hidden_regex: &Option<RegexSet>,
    default_regex: &Option<RegexSet>,
) -> Option<Nlist> {
    if !nlist.is_global() {
        return None;
    }
    if default_regex.is_some() && default_regex.as_ref().unwrap().is_match(name) {
        make_nlist_default(nlist, name, verbose)
    } else if hidden_regex.is_some() && hidden_regex.as_ref().unwrap().is_match(name) {
        make_nlist_hidden(nlist, name, verbose)
    } else {
        None
    }
}

pub fn run(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let verbose = matches.is_present("verbose");
    let hidden_regex = matches.values_of("hidden").map(RegexSet::new).transpose()?;
    let default_regex = matches
        .values_of("default")
        .map(RegexSet::new)
        .transpose()?;
    let mut rename_map = HashMap::new();
    if let Some(rename) = matches.values_of("rename") {
        let original = rename.clone().step_by(2);
        let renamed = rename.skip(1).step_by(2);
        for (old, new) in original.zip(renamed) {
            if new.len() > old.len() {
                return Err(Box::new(Error::Message(format!("Replacement symbol names cannot have more characters than the original name. Symbol '{}' cannot be renamed to '{}'.", old, new))));
            }
            rename_map.insert(old.to_string(), new.to_string());
        }
    }

    let transform: Box<backend::object::ObjectTransform<crate::error::Error>> =
        Box::new(move |bytes, object| {
            let mut patches = Vec::new();
            match object {
                backend::object::Object::Elf(elf) => {
                    if let Some(iter) = backend::elf::SymtabIter::symtab_from_elf(bytes, &elf)? {
                        for (ref name, ref sym) in
                            iter.collect::<backend::error::Result<Vec<_>>>()?
                        {
                            let (new_name, new_sym) = if let Some(name) = name {
                                let new_name = rename_map.get(*name.deref());
                                let new_sym = change_sym_vis(
                                    sym,
                                    name,
                                    verbose,
                                    &hidden_regex,
                                    &default_regex,
                                );
                                (new_name, new_sym)
                            } else {
                                (None, None)
                            };
                            if let (Some(name), Some(new_name)) = (name, new_name) {
                                // Resize the new name to match the old name, extending with NUL bytes as required.
                                // Since we checked that the new length is shorter than or equal to the old length
                                // above, this will only be extending the length.
                                let mut new_name_bytes = new_name.as_bytes().to_vec();
                                new_name_bytes.resize(name.len(), 0);
                                patches.push(name.patch_with_bytes(&new_name_bytes)?);
                            }
                            if let Some(new_sym) = new_sym {
                                patches.push(sym.patch_with(new_sym)?);
                            }
                        }
                    }
                }
                backend::object::Object::MachO(mach) => {
                    if let Some(iter) = backend::mach::SymtabIter::from_mach(bytes, &mach) {
                        for (ref name, ref nlist) in
                            iter.collect::<backend::error::Result<Vec<_>>>()?
                        {
                            let (new_name, new_nlist) = if let Some(name) = name {
                                let new_name = rename_map.get(*name.deref());
                                let new_nlist = change_nlist_vis(
                                    nlist,
                                    name,
                                    verbose,
                                    &hidden_regex,
                                    &default_regex,
                                );
                                (new_name, new_nlist)
                            } else {
                                (None, None)
                            };
                            if let (Some(name), Some(new_name)) = (name, new_name) {
                                // Resize the new name to match the old name, extending with NUL bytes as required.
                                // Since we checked that the new length is shorter than or equal to the old length
                                // above, this will only be extending the length.
                                let mut new_name_bytes = new_name.as_bytes().to_vec();
                                new_name_bytes.resize(name.len(), 0);
                                patches.push(name.patch_with_bytes(&new_name_bytes)?);
                            }
                            if let Some(new_nlist) = new_nlist {
                                patches.push(nlist.patch_with(new_nlist)?);
                            }
                        }
                    }
                }
            }
            Ok(patches)
        });

    let mut object = Vec::new();
    std::fs::File::open(matches.value_of("INPUT").unwrap())?.read_to_end(&mut object)?;
    backend::object::transform_object(&mut object, &transform)?;
    std::fs::File::create(matches.value_of("OUTPUT").unwrap())?.write_all(&object)?;
    Ok(())
}
