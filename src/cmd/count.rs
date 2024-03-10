static USAGE: &str = r#"
Prints a count of the number of records in the CSV data.

Note that the count will not include the header row (unless --no-headers is
given).

For examples, see https://github.com/jqnatividad/qsv/blob/master/tests/test_count.rs.

Usage:
    qsv count [options] [<input>]
    qsv count --help

count options:
    -H, --human-readable   Comma separate row count.
    --width                Also return the length of the longest record.
                           The count and width are separated by a semicolon.

                           WHEN THE POLARS FEATURE IS ENABLED:
    --no-polars            Use the regular single-threaded, streaming CSV reader instead of
                           the much faster Polars multi-threaded, mem-mapped CSV reader.
                           Use this when you encounter issues when counting with the
                           Polars CSV reader. The regular reader is slower but can read any
                           valid CSV file of any size.
    --low-memory           Use the Polars CSV Reader's low-memory mode. This
                           mode is slower but uses less memory.


Common options:
    -h, --help             Display this message
    -f, --flexible         Do not validate if the CSV has different number of
                           fields per record, increasing performance when counting
                           without an index. Automatically enabled when --width is set.
    -n, --no-headers       When set, the first row will be included in
                           the count.
"#;

use log::info;
use serde::Deserialize;

use crate::{config::Config, util, CliResult};

#[derive(Deserialize)]
struct Args {
    arg_input:           Option<String>,
    flag_human_readable: bool,
    flag_width:          bool,
    flag_no_polars:      bool,
    flag_low_memory:     bool,
    flag_flexible:       bool,
    flag_no_headers:     bool,
}

pub fn run(argv: &[&str]) -> CliResult<()> {
    let args: Args = util::get_args(USAGE, argv)?;
    let conf = Config::new(&args.arg_input)
        .no_headers(args.flag_no_headers)
        // we also want to count the quotes when computing width
        .quoting(!args.flag_width)
        // and ignore differing column counts as well
        .flexible(args.flag_width || args.flag_flexible);

    // this comment left here for Logging.md example
    // log::debug!(
    //     "input: {:?}, no_header: {}",
    //     (args.arg_input).clone().unwrap(),
    //     &args.flag_no_headers,
    // );

    let (count, width) = if args.flag_width {
        count_input(&conf, args.flag_width)?
    } else {
        let index_status = conf.indexed().unwrap_or_else(|_| {
            info!("index is stale");
            None
        });
        match index_status {
            Some(idx) => {
                info!("index used");
                (idx.count(), 0)
            },
            None => {
                #[cfg(feature = "polars")]
                {
                    // if --no-polars or --width is set or its a snappy compressed file, use the
                    // regular CSV reader
                    if args.flag_no_polars || args.flag_width || conf.is_snappy() {
                        count_input(&conf, args.flag_width)?
                    } else {
                        polars_count_input(&conf, args.flag_low_memory)?
                    }
                }
                #[cfg(not(feature = "polars"))]
                {
                    count_input(&conf, args.flag_width)?
                }
            },
        }
    };

    if args.flag_human_readable {
        use indicatif::HumanCount;

        if args.flag_width {
            woutinfo!("{};{}", HumanCount(count), HumanCount(width as u64));
        } else {
            woutinfo!("{}", HumanCount(count));
        }
    } else if args.flag_width {
        woutinfo!("{count};{width}");
    } else {
        woutinfo!("{count}");
    }
    Ok(())
}

fn count_input(
    conf: &Config,
    compute_width: bool,
) -> Result<(u64, usize), crate::clitypes::CliError> {
    let mut rdr = conf.reader()?;
    let mut count = 0_u64;
    let mut max_width = 0_usize;
    let mut record_numdelimiters = 0_usize;
    let mut record = csv::ByteRecord::new();

    if compute_width {
        let mut curr_width;

        // read the first record to get the number of delimiters
        // and the width of the first record
        rdr.read_byte_record(&mut record)?;
        max_width = record.as_slice().len();
        count = 1;

        // number of delimiters is number of fields minus 1
        // we subtract 1 because the last field doesn't have a delimiter
        record_numdelimiters = record.len().saturating_sub(1);

        while rdr.read_byte_record(&mut record)? {
            count += 1;

            curr_width = record.as_slice().len();
            if curr_width > max_width {
                max_width = curr_width;
            }
        }
    } else {
        while rdr.read_byte_record(&mut record)? {
            count += 1;
        }
    }
    // record_numdelimiters is a count of the delimiters
    // which we also want to count when returning width
    Ok((count, max_width + record_numdelimiters))
}

#[cfg(feature = "polars")]
fn polars_count_input(
    conf: &Config,
    low_memory: bool,
) -> Result<(u64, usize), crate::clitypes::CliError> {
    use polars::prelude::*;

    log::info!("using polars");
    let mut comment_char = String::new();
    let temp_char;

    let comment_prefix = if let Some(c) = conf.comment {
        comment_char.push(c as char);
        temp_char = comment_char.to_string();
        Some(temp_char.as_str())
    } else {
        None
    };

    let df = if conf.is_stdin() {
        let mut temp_file = tempfile::Builder::new().suffix(".csv").tempfile()?;
        let stdin = std::io::stdin();
        let mut stdin_handle = stdin.lock();
        std::io::copy(&mut stdin_handle, &mut temp_file)?;
        drop(stdin_handle);

        let path = temp_file
            .into_temp_path()
            .as_os_str()
            .to_string_lossy()
            .to_string();

        CsvReader::from_path(path)?
            .with_comment_prefix(comment_prefix)
            .has_header(!conf.no_headers)
            .truncate_ragged_lines(conf.flexible)
            .low_memory(low_memory)
            .finish()?
    } else {
        let csv_path = conf.path.as_ref().unwrap().to_str().unwrap().to_string();
        polars::io::csv::CsvReader::from_path(csv_path)?
            .with_comment_prefix(comment_prefix)
            .has_header(!conf.no_headers)
            .truncate_ragged_lines(conf.flexible)
            .low_memory(low_memory)
            .finish()?
    };
    let count = df.height() as u64;
    Ok((count, 0))
}
