use clap::{arg, Command};

pub fn frequency_cmd() -> Command {
    Command::new("frequency").args([
        arg!(--select),
        arg!(--limit),
        arg!(--"unq-limit"),
        arg!(--"lmt-threshold"),
        arg!(--"pct-dec-places"),
        arg!(--"other-sorted"),
        arg!(--"other-text"),
        arg!(--asc),
        arg!(--"no-trim"),
        arg!(--"no-nulls"),
        arg!(--"ignore-case"),
        arg!(--"stats-mode"),
        arg!(--"all-unique-text"),
        arg!(--jobs),
        arg!(--output),
        arg!(--"no-headers"),
        arg!(--delimiter),
        arg!(--memcheck),
    ])
}
