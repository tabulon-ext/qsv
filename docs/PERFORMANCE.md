# Performance Tuning

## CPU Optimization

Modern CPUs have various features that the Rust compiler can take advantage
of to increase performance. If you want the compiler to take advantage of these
CPU-specific speed-ups, set this environment variable **BEFORE** installing/compiling qsv:

On Linux and macOS:

```bash
export CARGO_BUILD_RUSTFLAGS='-C target-cpu=native'
```

On Windows Powershell:

```powershell
$env:CARGO_BUILD_RUSTFLAGS='-C target-cpu=native'
```

Do note though that the resulting binary will only run on machines with the
same architecture as the machine you installed/compiled from.
To find out your CPU architecture and other valid values for `target-cpu`:

```bash
rustc --print target-cpus

# to find out what CPU features are used by the Rust compiler WITHOUT specifying target-cpu
rustc --print cfg | grep -i target_feature

# to find out what additional CPU features will be used by the Rust compiler when you specify target-cpu=native
rustc --print cfg -C target-cpu=native | grep -i target_feature

# to get a short explanation of each CPU target-feature
rustc --print target-features
```

## Memory Allocator

By default, qsv uses an alternative allocator - [mimalloc](https://github.com/microsoft/mimalloc),
a performance-oriented allocator from Microsoft.
If you want to use the standard allocator, use the `--no-default-features` flag
when installing/compiling qsv, e.g.:

```bash
cargo install qsv --path . --no-default-features
```

or

```bash
cargo build --release --no-default-features
```

To find out what memory allocator qsv is using, run `qsv --version`. After the qsv version number, the allocator used is displayed ("`standard`" or "`mimalloc`"). Note that mimalloc is not supported on the `x86_64-pc-windows-gnu` and `arm` targets, and you'll need to use the "standard" allocator on those platforms.

## Buffer size

Depending on your filesystem's configuration (e.g. block size, file system type, writing to remote file systems (e.g. sshfs, efs, nfs),
SSD or rotating magnetic disks, etc.), you can also fine-tune qsv's read/write buffers.

By default, the read buffer size is set to [16k](https://github.com/jqnatividad/qsv/blob/master/src/config.rs#L16), you can change it by setting the environment
variable `QSV_RDR_BUFFER_CAPACITY` in bytes.

The same is true with the write buffer (default: 64k) with the `QSV_WTR_BUFFER_CAPACITY` environment variable.

## Multithreading

Several commands support multithreading - `stats`, `frequency`, `schema` and `split` (when an index is available); `dedup`, `extsort`, `sort` and `validate` (no index required).

qsv will automatically spawn parallel jobs equal to the detected number of logical processors. Should you want to manually override this, use the `--jobs` command-line option or the `QSV_MAX_JOBS` environment variable.

To find out your jobs setting, call `qsv --version`. The second to the last number is the number of jobs qsv will use for multithreaded commands. The last number is the number of logical processors detected by qsv.  For example:

```
$ qsv --version
qsv 0.46.1-mimalloc-apply;fetch;generate;lua;python;-16-16
```

Shows that I'm running qsv version 0.46.1, with the `mimalloc` allocator (instead of `standard`), and I have the `apply`, `fetch`, `generate`, `lua` and `python` features enabled, and qsv will be using 16 logical processors out of 16 detected when running multithreaded commands.

## Caching
The `apply geocode` command [memoizes](https://en.wikipedia.org/wiki/Memoization) otherwise expensive geocoding operations and will report its cache hit rate. `apply geocode` memoization, however, is not persistent across sessions.

The `fetch` command also memoizes expensive REST API calls with its optional Redis support. It effectively has a persistent cache as the default time-to-live (TTL) before a Redis cache entry is expired is 28 days and Redis entries are persisted across restarts. Redis cache settings can be fine-tuned with the `QSV_REDIS_CONNECTION_STRING`, `QSV_REDIS_TTL_SECONDS` and `QSV_REDIS_TTL_REFRESH` environment variables.

## UTF-8 Encoding for Performance
[Rust strings are utf-8 encoded](https://doc.rust-lang.org/std/string/struct.String.html). As a result, qsv **requires** UTF-8 encoded files, primarily, for performance. It makes extensive use of [`str::from_utf8_unchecked`](https://doc.rust-lang.org/stable/std/str/fn.from_utf8_unchecked.html) to skip utf-8 validation that [`str::from_utf8`](https://doc.rust-lang.org/stable/std/str/fn.from_utf8.html) will otherwise incur everytime raw bytes are converted to string.

For the most part, this shouldn't be a problem as UTF-8 is the de facto encoding standard. Should you need to process a CSV file with a different encoding, use the `input` command to "transcode" to UTF-8.

## Nightly Release Builds
Pre-built binaries compiled using Rust Nightly/Unstable are also [available for download](https://github.com/jqnatividad/qsv/releases/latest). These binaries are optimized for size and speed:

* compiled with the current Rust nightly/unstable at the time of release.
* stdlib is compiled from source, instead of using the pre-built stdlib. This ensures stdlib is compiled with all of qsv's release settings
  (link time optimization, opt-level, codegen-units, panic=abort, etc.). This is why we only have nightly release builds for select platforms 
  (the platform of GitHub's action runners), as we need access to the "native hardware" and cannot cross-compile stdlib to other platforms.
* set `panic=abort` - removing panic-handling/formatting and backtrace code, making for smaller binaries.
* enables unstable/nightly features on `regex`, `rand` and `pyo3` crates, that unlock performance/SIMD features on those crates.

Despite the 'unstable' label, these binaries are actually quite stable, given how [Rust is made](https://doc.rust-lang.org/book/appendix-07-nightly-rust.html),
and the fact that qsv itself doesn't actually use any unstable feature flags, beyond activating the 'unstable' features in the `rand`, `regex` and `pyo3` crates, which is really more about performance (that's why we can still compile with Rust stable). You only really loose the backtrace messages when qsv panics.

If you need to maximize speed/performance - use the nightly builds. If you prefer a "safer", rock-solid experience, use the stable builds.

If you want to really squeeze every little bit of performance from qsv, build it locally like how the Nightly Release Builds are built.
Doing so will ensure CPU features are tailored to your hardware and you're using the latest Rust nightly.
For example, on Ubuntu 22.04 LTS Linux:

```
rustup default nightly
rustup update
export RUSTFLAGS='-C target-cpu=native'

# to build qsv on nightly with all features. The binary will be in the target/release-nightly folder.
cargo build --profile release-nightly --bin qsv -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort --features full,apply,generate,lua,fetch,foreach,python,nightly --target x86_64-unknown-linux-gnu

# to build qsvlite
cargo build --profile release-nightly --bin qsvlite -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort --features lite,nightly --target x86_64-unknown-linux-gnu

# to build qsvdp
cargo build --profile release-nightly --bin qsvdp -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort --features datapusher_plus,nightly --target x86_64-unknown-linux-gnu
```

## Benchmarking for Performance

Use and fine-tune the [benchmark script](scripts/benchmark-basic.sh) when tweaking qsv's performance to your environment.
Don't be afraid to change the benchmark data and the qsv commands to something that is more representative of your
workloads.

Use the generated benchmark TSV files to meter and compare performance across platforms. You'd be surprised how performance varies
across environments - e.g. qsv's `join` performs abysmally on Windows's WSL running Ubuntu 20.04 LTS, taking 172.44 seconds.
On the same machine, running in a VirtualBox VM at that with the same Ubuntu version, `join` was done in 1.34 seconds -
two orders of magnitude faster!

However, `stats` performs two times faster on WSL vs the VirtualBox VM - 2.80 seconds vs 5.33 seconds for the `stats_index` benchmark.
