# serialbench-rs

The Rust harness of the cross-language serialbench family. Measures the
canonical fixtures (https://github.com/serialbench/fixtures) with the native
tris Rust bindings — [leptris](https://crates.io/crates/leptris) (XML 1.0,
XPath 1.0), [yeptris](https://crates.io/crates/yeptris) (YAML 1.2),
[teptris](https://crates.io/crates/teptris) (TOML 1.1) — writing the shared
data schema (`runs/{date}/{platform}-rust-{rustc}.{format}.yaml`).

## Build and run

The bindings link the C engines at build time; point them at built
libraries (or install them on the library path):

```sh
export LEPTRIS_LIB_PATH=/path/to/libleptris.dylib   # or .so
export YEPTRIS_LIB_PATH=/path/to/libyeptris.dylib
export TEPTRIS_LIB_PATH=/path/to/libteptris.dylib
export RUSTFLAGS="-C link-arg=-Wl,-rpath,$LIBDIR"   # runtime resolution
cargo build --release

git clone --depth 1 https://github.com/serialbench/fixtures.git fixtures
./target/release/serialbench-rs fixtures results/xml/results.yaml [format]
```

Formats: `xml yaml toml` (default: all). Operations: parsing, generation,
xpath (xml). Iteration discipline matches the family: warmup 3, then
small:10 / medium:3 / large:1.

## CI

`.github/workflows/benchmark.yml` runs weekly (ubuntu x64/arm + macos-26):
builds the three C engines from source, builds the harness with an rpath
into them, then per-format checkpoints — benchmark, artifact upload, push
to `serialbench/data` via the Contents API, and a completeness check that
fails the leg so autoheal fills the gaps.

`runtime_version` is the rustc version (`1.98.0`); the data filename and
the site's env key derive from it.
