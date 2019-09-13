use clap::{App, Arg, ArgMatches, SubCommand};
use goblin::elf::sym::{Sym, STV_DEFAULT, STV_HIDDEN};
use goblin::mach::symbols::Nlist;
use objedit::elf::ElfTransform;
use objedit::error::Result;
use objedit::mach::MachTransform;

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

pub fn run(matches: &ArgMatches, verbosity: u64) -> Result<()> {
    let mut elf = ElfTransform::new();
    let mut mach = MachTransform::new();
    if matches.is_present("all-hidden") {
        elf.with_symtab_transform(Box::new(|_, sym| (None, Some(make_sym_hidden(sym)))));
    }
    if matches.is_present("all-default") {
        elf.with_symtab_transform(Box::new(|_, sym| (None, Some(make_sym_default(sym)))));
    }
    if let Some(regexes) = matches.values_of("hidden") {}
    if let Some(regexes) = matches.values_of("default") {}
    Ok(())
}
