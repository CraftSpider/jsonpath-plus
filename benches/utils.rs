
use criterion::Criterion;
use pprof::criterion::{PProfProfiler, Output};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct BenchPaths {
    pub name: String,
    pub path: String,
    pub input: Option<Value>,
}

impl BenchPaths {
    pub fn read() -> Vec<BenchPaths> {
        serde_json::from_reader(std::fs::File::open("benches/bench_paths.json").unwrap()).unwrap()
    }
}

pub fn config_criterion() -> Criterion {
    Criterion::default()
        .with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)))
        .configure_from_args()
}
