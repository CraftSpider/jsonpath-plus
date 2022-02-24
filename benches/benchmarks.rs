use criterion::{criterion_main, BenchmarkId};
use jsonpath_plus::JsonPath;

mod utils;

use utils::{config_criterion, BenchPaths};

pub fn parse() {
    let mut c = config_criterion();
    let mut group = c.benchmark_group("JsonPath::compile");
    for path in BenchPaths::read() {
        group.bench_with_input(
            BenchmarkId::from_parameter(path.name),
            &*path.path,
            |b, p| b.iter(|| JsonPath::compile(p)),
        );
    }
    group.finish()
}

pub fn find() {
    let mut c = config_criterion();
    let mut group = c.benchmark_group("JsonPath::find");
    for path in BenchPaths::read() {
        let input = match &path.input {
            Some(input) => input,
            None => continue,
        };
        let json_path = JsonPath::compile(&path.path).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(path.name), input, |b, val| {
            b.iter(|| json_path.find(val))
        });
    }
    group.finish()
}

pub fn find_paths() {
    let mut c = config_criterion();
    let mut group = c.benchmark_group("JsonPath::find_paths");
    for path in BenchPaths::read() {
        let input = match &path.input {
            Some(input) => input,
            None => continue,
        };
        let json_path = JsonPath::compile(&path.path).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(path.name), input, |b, val| {
            b.iter(|| json_path.find_paths(val))
        });
    }
    group.finish()
}

criterion_main!(parse, find, find_paths);
