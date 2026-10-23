//! serialbench-rs: cross-language benchmark harness, Rust runtime.
//! Measures the canonical fixtures with the native tris Rust bindings
//! (leptris / yeptris / teptris), writing the shared data-repo schema.
//!
//! Usage: serialbench-rs <fixtures-dir> <out-results-yaml> [format]
//! Formats: xml yaml toml (default: all).

use std::fmt::Write as _;
use std::time::Instant;

mod leptris_bench {
    // Adapter version, kept in lockstep with the Cargo.toml dependency.
    pub const VERSION: &str = "1.4.1";
    pub const XPATH_QUERIES: [&str; 3] = [
        "//user | //record",
        "//user[@id='101']",
        "//preferences/theme",
    ];
}

struct Row {
    adapter: &'static str,
    format: &'static str,
    size: &'static str,
    op: &'static str,
    time_per_iteration: f64,
    iterations_per_second: f64,
}

struct Ctx {
    rows: Vec<Row>,
    sizes: [&'static str; 3],
    iters: [usize; 3],
}

const WARMUP: usize = 3;

impl Ctx {
    fn bench<F: FnMut()>(&mut self, adapter: &'static str, format: &'static str, op: &'static str, size_idx: usize, mut work: F) {
        let size = self.sizes[size_idx];
        let iters = self.iters[size_idx];
        for _ in 0..WARMUP {
            work();
        }
        let start = Instant::now();
        for _ in 0..iters {
            work();
        }
        let elapsed = start.elapsed().as_secs_f64();
        self.rows.push(Row {
            adapter,
            format,
            size,
            op,
            time_per_iteration: elapsed / iters as f64,
            iterations_per_second: iters as f64 / elapsed,
        });
    }
}

fn read_fixture(dir: &str, size: &str, ext: &str) -> Option<Vec<u8>> {
    std::fs::read(format!("{dir}/{size}.{ext}")).ok()
}

fn bench_json(ctx: &mut Ctx, dir: &str, size_idx: usize) {
    let size = ctx.sizes[size_idx];
    let Some(data) = read_fixture(dir, size, "json") else {
        eprintln!("missing {size}.json");
        return;
    };
    ctx.bench("serde-json", "json", "parsing", size_idx, || {
        let _v: serde_json::Value = serde_json::from_slice(&data).expect("json parse");
    });
    ctx.bench("serde-json", "json", "generation", size_idx, || {
        let v: serde_json::Value = serde_json::from_slice(&data).expect("json parse");
        let _s = serde_json::to_string(&v).expect("json emit");
    });
    ctx.bench("simd-json", "json", "parsing", size_idx, || {
        // to_owned_value takes &mut [u8]: the API's in-situ parse on an
        // owned copy of the input.
        let mut buf = data.clone();
        let _v = simd_json::to_owned_value(&mut buf).expect("simd json parse");
    });
    ctx.bench("simd-json", "json", "generation", size_idx, || {
        let mut buf = data.clone();
        let v = simd_json::to_owned_value(&mut buf).expect("simd json parse");
        let _s = v.to_string();
    });
}

fn bench_xml(ctx: &mut Ctx, dir: &str, size_idx: usize) {
    let size = ctx.sizes[size_idx];
    let Some(data) = read_fixture(dir, size, "xml") else {
        eprintln!("missing {size}.xml");
        return;
    };
    ctx.bench("leptris", "xml", "parsing", size_idx, || {
        let _doc = leptris::Document::parse(&data).expect("xml parse");
    });
    ctx.bench("leptris", "xml", "generation", size_idx, || {
        let doc = leptris::Document::parse(&data).expect("xml parse");
        let _out = doc.serialize().expect("xml serialize");
    });
    ctx.bench("leptris", "xml", "xpath", size_idx, || {
        let doc = leptris::Document::parse(&data).expect("xml parse");
        for q in leptris_bench::XPATH_QUERIES {
            let _hits = doc.xpath(q).expect("xpath");
        }
    });
    ctx.bench("quick-xml", "xml", "parsing", size_idx, || {
        use quick_xml::events::Event;
        let mut reader = quick_xml::Reader::from_str(std::str::from_utf8(&data).expect("utf8"));
        let mut events = 0;
        loop {
            match reader.read_event() {
                Ok(Event::Eof) => break,
                Ok(_) => events += 1,
                Err(e) => panic!("quick-xml parse: {e}"),
            }
        }
        assert!(events > 0, "quick-xml: no events");
    });
    ctx.bench("roxmltree", "xml", "parsing", size_idx, || {
        let doc = roxmltree::Document::parse(std::str::from_utf8(&data).expect("utf8")).expect("roxmltree parse");
        assert!(doc.descendants().next().is_some());
    });
}

fn bench_yaml(ctx: &mut Ctx, dir: &str, size_idx: usize) {
    let size = ctx.sizes[size_idx];
    let Some(data) = read_fixture(dir, size, "yaml") else {
        eprintln!("missing {size}.yaml");
        return;
    };
    ctx.bench("yeptris", "yaml", "parsing", size_idx, || {
        let _doc = yeptris::Document::parse(&data).expect("yaml parse");
    });
    ctx.bench("yeptris", "yaml", "generation", size_idx, || {
        let doc = yeptris::Document::parse(&data).expect("yaml parse");
        let _out = doc.serialize_string().expect("yaml serialize");
    });
    ctx.bench("yaml-rust2", "yaml", "parsing", size_idx, || {
        let _docs = yaml_rust2::YamlLoader::load_from_str(std::str::from_utf8(&data).expect("utf8")).expect("yaml parse");
    });
    ctx.bench("yaml-rust2", "yaml", "generation", size_idx, || {
        let docs = yaml_rust2::YamlLoader::load_from_str(std::str::from_utf8(&data).expect("utf8")).expect("yaml parse");
        let mut out = String::new();
        yaml_rust2::YamlEmitter::new(&mut out)
            .dump(&docs[0])
            .expect("yaml emit");
    });
}

fn bench_toml(ctx: &mut Ctx, dir: &str, size_idx: usize) {
    let size = ctx.sizes[size_idx];
    let Some(data) = read_fixture(dir, size, "toml") else {
        eprintln!("missing {size}.toml");
        return;
    };
    ctx.bench("teptris", "toml", "parsing", size_idx, || {
        let _v = teptris::loads(&data).expect("toml parse");
    });
    ctx.bench("teptris", "toml", "generation", size_idx, || {
        let v = teptris::loads(&data).expect("toml parse");
        let _out = teptris::dump(&v).expect("toml dump");
    });
    ctx.bench("toml", "toml", "parsing", size_idx, || {
        let _v: toml::Value = toml::from_str(std::str::from_utf8(&data).expect("utf8")).expect("toml parse");
    });
    ctx.bench("toml", "toml", "generation", size_idx, || {
        let v: toml::Value = toml::from_str(std::str::from_utf8(&data).expect("utf8")).expect("toml parse");
        let _s = toml::to_string(&v).expect("toml emit");
    });
}

fn os_family() -> &'static str {
    match std::env::consts::OS {
        "macos" => "macos",
        "linux" => "linux",
        "windows" => "windows",
        other => Box::leak(other.to_string().into_boxed_str()),
    }
}

fn arch() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x86_64",
        other => Box::leak(other.to_string().into_boxed_str()),
    }
}

fn runtime_version() -> &'static str {
    Box::leak(env!("SERIALBENCH_RUSTC_VERSION").to_string().into_boxed_str())
}

fn platform_label() -> String {
    std::env::var("SERIALBENCH_PLATFORM").unwrap_or_else(|_| format!("local-{}", arch()))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: serialbench-rs <fixtures-dir> <out-yaml> [format]");
        std::process::exit(2);
    }
    let fixtures = &args[1];
    let out_path = &args[2];
    let want = args.get(3).map(|s| s.as_str());

    let ctx_inner = std::cell::RefCell::new(Ctx {
        rows: Vec::new(),
        sizes: ["small", "medium", "large"],
        iters: [10, 3, 1],
    });

    for size_idx in 0..3 {
        match want {
            Some("xml") | None => bench_xml(&mut ctx_inner.borrow_mut(), fixtures, size_idx),
            _ => {}
        }
        match want {
            Some("yaml") | None => bench_yaml(&mut ctx_inner.borrow_mut(), fixtures, size_idx),
            _ => {}
        }
        match want {
            Some("toml") | None => bench_toml(&mut ctx_inner.borrow_mut(), fixtures, size_idx),
            _ => {}
        }
        match want {
            Some("json") | None => bench_json(&mut ctx_inner.borrow_mut(), fixtures, size_idx),
            _ => {}
        }
    }

    let ctx = ctx_inner.into_inner();
    let rustc = runtime_version();
    let platform = platform_label();

    let mut out = String::new();
    out.push_str("platform:\n");
    let _ = writeln!(out, "  platform_string: {platform}-rust-{rustc}");
    out.push_str("  kind: native\n");
    let _ = writeln!(out, "  os: {}", os_family());
    let _ = writeln!(out, "  arch: {}", arch());
    out.push_str("  runtime: rust\n");
    let _ = writeln!(out, "  runtime_version: {rustc}");
    out.push_str("benchmark_config:\n");
    out.push_str("  name: rust-full\n");
    out.push_str("  formats: [json, xml, yaml, toml]\n");
    out.push_str("  operations: [parsing, generation, xpath]\n");
    out.push_str("benchmark_result:\n");
    out.push_str("  serializers:\n");
    let _ = writeln!(out, "    - {{name: leptris, format: xml, version: '{}', features: {{xpath: true}}}}", leptris_bench::VERSION);
    let _ = writeln!(out, "    - {{name: yeptris, format: yaml, version: '0.6.3', features: {{}}}}");
    let _ = writeln!(out, "    - {{name: teptris, format: toml, version: '0.1.6', features: {{}}}}");
    for (name, fmt, ver) in [
        ("serde-json", "json", "1.0.154"),
        ("simd-json", "json", "0.18.1"),
        ("quick-xml", "xml", "0.42.0"),
        ("roxmltree", "xml", "0.21.1"),
        ("yaml-rust2", "yaml", "0.13.0"),
        ("toml", "toml", "1.1.8"),
    ] {
        let _ = writeln!(out, "    - {{name: {name}, format: {fmt}, version: '{ver}', features: {{}}}}");
    }
    for op in ["parsing", "generation", "xpath"] {
        out.push_str(&format!("  {op}:\n"));
        for row in &ctx.rows {
            if row.op == op {
                let _ = writeln!(
                    out,
                    "    - {{adapter: {}, format: {}, data_size: {}, time_per_iteration: {:.9}, iterations_per_second: {:.6}}}",
                    row.adapter, row.format, row.size, row.time_per_iteration, row.iterations_per_second
                );
            }
        }
    }

    std::fs::write(out_path, out).expect("write results");
    println!("rows: {}", ctx.rows.len());
}

#[cfg(test)]
mod tests {
    #[test]
    fn runtime_version_captured() {
        assert!(super::runtime_version().chars().next().unwrap().is_ascii_digit());
    }

    #[test]
    fn platform_defaults_local() {
        // SERIALBENCH_PLATFORM unset in tests -> local-<arch>
        if std::env::var("SERIALBENCH_PLATFORM").is_err() {
            assert!(super::platform_label().starts_with("local-"));
        }
    }
}
