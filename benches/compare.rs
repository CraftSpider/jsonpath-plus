
mod utils;

use criterion::{criterion_main, BenchmarkId};
use utils::{config_criterion, BenchPaths};

pub fn jsonpath_plus() {
    use jsonpath_plus::JsonPath;

    let mut c = config_criterion();
    let mut group = c.benchmark_group("jsonpath_plus");
    for path in BenchPaths::read() {
        group.bench_with_input(
            BenchmarkId::new("parse", path.name.clone()),
            &*path.path,
            |b, p| b.iter(|| JsonPath::compile(p)),
        );

        if let Some(input) = path.input {
            let json_path = JsonPath::compile(&path.path)
                .unwrap();
            group.bench_with_input(
                BenchmarkId::new("find", path.name),
                &input,
                |b, p| b.iter(|| json_path.find(p)),
            );
        }

    }
    group.finish()
}

pub fn jsonpath_lib() {
    use jsonpath_lib::Compiled;

    let mut c = config_criterion();
    let mut group = c.benchmark_group("jsonpath_lib");
    for path in BenchPaths::read() {
        if let Err(e) = Compiled::compile(&path.path) {
            eprintln!("jsonpath_lib doesn't support path: \"{}\", error {}", path.path, e);
            continue;
        }

        group.bench_with_input(
            BenchmarkId::new("parse", path.name.clone()),
            &*path.path,
            |b, p| b.iter(|| Compiled::compile(p)),
        );

        if let Some(input) = path.input {
            let json_path = Compiled::compile(&path.path).unwrap();
            group.bench_with_input(
                BenchmarkId::new("find", path.name),
                &input,
                |b, p| b.iter(|| json_path.select(p)),
            );
        }

    }
    group.finish()
}

criterion_main!(jsonpath_plus, jsonpath_lib);
