use clap::{arg, Command};

pub fn searchset_cmd() -> Command {
    Command::new("searchset").args([
        arg!(--"ignore-case"),
        arg!(--literal),
        arg!(--select),
        arg!(--"invert-match"),
        arg!(--unicode),
        arg!(--flag),
        arg!(--"flag-matches-only"),
        arg!(--"unmatched-output"),
        arg!(--quick),
        arg!(--count),
        arg!(--json),
        arg!(--"size-limit"),
        arg!(--"dfa-size-limit"),
        arg!(--"not-one"),
        arg!(--output),
        arg!(--"no-headers"),
        arg!(--delimiter),
        arg!(--progressbar),
        arg!(--quiet),
    ])
}
