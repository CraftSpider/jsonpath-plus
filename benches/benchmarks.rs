use criterion::{criterion_main, BenchmarkId, Criterion};
use jsonpath_plus::JsonPath;
use pprof::criterion::{Output, PProfProfiler};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct BenchPaths {
    name: String,
    path: String,
    input: Option<Value>,
}

impl BenchPaths {
    fn read() -> Vec<BenchPaths> {
        serde_json::from_reader(std::fs::File::open("benches/bench_paths.json").unwrap()).unwrap()
    }
}

fn config_criterion() -> Criterion {
    Criterion::default()
        .with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)))
        .configure_from_args()
}

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

pub fn eval() {
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

criterion_main!(parse, eval);
