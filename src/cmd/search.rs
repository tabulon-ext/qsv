use regex::bytes::RegexBuilder;
use std::env;

use crate::config::{Config, Delimiter};
use crate::select::SelectColumns;
use crate::util;
use crate::CliResult;
use serde::Deserialize;

static USAGE: &str = "
Filters CSV data by whether the given regex matches a row.

The regex is applied to each field in each row, and if any field matches,
then the row is written to the output. The columns to search can be limited
with the '--select' flag (but the full row is still written to the output if
there is a match).

Usage:
    qsv search [options] <regex> [<input>]
    qsv search --help

search options:
    -i, --ignore-case      Case insensitive search. This is equivalent to
                           prefixing the regex with '(?i)'.
    -s, --select <arg>     Select the columns to search. See 'qsv select -h'
                           for the full syntax.
    -v, --invert-match     Select only rows that did not match
    -u, --unicode          Enable unicode support. When enabled, character classes
                           will match all unicode word characters instead of only
                           ASCII word characters. Decreases performance.    

Common options:
    -h, --help             Display this message
    -o, --output <file>    Write output to <file> instead of stdout.
    -n, --no-headers       When set, the first row will not be interpreted
                           as headers. (i.e., They are not searched, analyzed,
                           sliced, etc.)
    -d, --delimiter <arg>  The field delimiter for reading CSV data.
                           Must be a single character. (default: ,)
    -f, --flag <column>    If given, the command will not filter rows
                           but will instead flag the found rows in a new
                           column named <column>, with the row numbers
                           of the matched rows.
";

#[derive(Deserialize)]
struct Args {
    arg_input: Option<String>,
    arg_regex: String,
    flag_select: SelectColumns,
    flag_output: Option<String>,
    flag_no_headers: bool,
    flag_delimiter: Option<Delimiter>,
    flag_invert_match: bool,
    flag_unicode: bool,
    flag_ignore_case: bool,
    flag_flag: Option<String>,
}

pub fn run(argv: &[&str]) -> CliResult<()> {
    let args: Args = util::get_args(USAGE, argv)?;
    let regex_unicode = match env::var("QSV_REGEX_UNICODE") {
        Ok(_) => true,
        Err(_) => args.flag_unicode,
    };
    let pattern = RegexBuilder::new(&*args.arg_regex)
        .case_insensitive(args.flag_ignore_case)
        .unicode(regex_unicode)
        .build()?;
    let rconfig = Config::new(&args.arg_input)
        .delimiter(args.flag_delimiter)
        .no_headers(args.flag_no_headers)
        .select(args.flag_select);

    let mut rdr = rconfig.reader()?;
    let mut wtr = Config::new(&args.flag_output).writer()?;

    let mut headers = rdr.byte_headers()?.clone();
    let sel = rconfig.selection(&headers)?;

    if let Some(column_name) = args.flag_flag.clone() {
        headers.push_field(column_name.as_bytes());
    }

    if !rconfig.no_headers {
        wtr.write_record(&headers)?;
    }
    let mut record = csv::ByteRecord::new();
    let mut flag_rowi: u64 = 1;
    #[allow(unused_assignments)]
    let mut matched_rows = String::with_capacity(20); // to save on allocs
    while rdr.read_byte_record(&mut record)? {
        let mut m = sel.select(&record).any(|f| pattern.is_match(f));
        if args.flag_invert_match {
            m = !m;
        }

        if args.flag_flag.is_some() {
            flag_rowi += 1;
            record.push_field(if m {
                matched_rows = flag_rowi.to_string();
                matched_rows.as_bytes()
            } else {
                b"0"
            });
            wtr.write_byte_record(&record)?;
        } else if m {
            wtr.write_byte_record(&record)?;
        }
    }
    Ok(wtr.flush()?)
}
