use std::{collections::HashMap, time::Duration};

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rust_extensions::sorted_vec::{EntityWithStrKey, SortedVecWithStrKey};
use uuid::Uuid;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TestStruct {
    pub id: String,
}

impl EntityWithStrKey for TestStruct {
    fn get_key(&self) -> &str {
        &self.id
    }
}

impl Default for TestStruct {
    fn default() -> Self {
        TestStruct {
            id: Uuid::new_v4().to_string(),
        }
    }
}
fn generate_init_data(
    amount: usize,
) -> (HashMap<String, TestStruct>, SortedVecWithStrKey<TestStruct>) {
    let mut data = SortedVecWithStrKey::new();
    let mut data2 = HashMap::new();
    for _ in 0..amount {
        let strc = TestStruct::default();

        data.insert_or_replace(strc.clone());
        data2.insert(strc.id.clone(), strc);
    }

    println!("generated data: {:?}", data.len());
    (data2, data)
}

pub fn search_bench(c: &mut Criterion) {
    let start_init_data = 1_000;

    let test_data = [
        generate_init_data(start_init_data),
        generate_init_data(start_init_data * 2),
        generate_init_data(start_init_data * 4),
        generate_init_data(start_init_data * 6),
        generate_init_data(start_init_data * 10),
    ];

    let mut group = c.benchmark_group("sorted_vec_vs_hashmap-search_bench");
    for data in test_data.iter() {
        let ids = data.0.keys().map(|x| x.as_str()).collect::<Vec<_>>();

        group.throughput(Throughput::Elements(data.1.len() as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("Vec {}", data.1.len())),
            &data.1.clone(),
            |b, src| {
                b.iter(|| search_for_all_elements_vec(src, &ids));
            },
        );

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("HashMap {}", data.1.len())),
            &data.0.clone(),
            |b, src| {
                b.iter(|| search_for_all_elements_hashmap(src, &ids));
            },
        );
    }
    group.finish();
}

fn search_for_all_elements_vec(src: &SortedVecWithStrKey<TestStruct>, ids: &[&str]) {
    for item in ids.iter() {
        let _ = src.get(item);
    }
}

fn search_for_all_elements_hashmap(src: &HashMap<String, TestStruct>, ids: &[&str]) {
    for item in ids.iter() {
        let _ = src.get(*item);
    }
}

// inserting

pub fn inset_bench(c: &mut Criterion) {
    let start_init_data = 1_000;

    let test_data = [
        generate_init_data(start_init_data),
        generate_init_data(start_init_data * 2),
        generate_init_data(start_init_data * 4),
        generate_init_data(start_init_data * 6),
        generate_init_data(start_init_data * 10),
    ];

    let mut group = c.benchmark_group("sorted_vec_vs_hashmap-inset_bench");
    for data in test_data.iter() {
        group.throughput(Throughput::Elements(data.1.len() as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("Vec {}", data.1.len())),
            &data.1.clone(),
            |b, src| {
                let item_to_insert = TestStruct::default();
                let mut src = src.clone();

                b.iter(|| src.insert_or_replace(item_to_insert.clone()));
            },
        );

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("HashMap {}", data.1.len())),
            &data.0.clone(),
            |b, src| {
                let item_to_insert = TestStruct::default();
                let mut src = src.clone();

                b.iter(|| src.insert(item_to_insert.id.clone(), item_to_insert.clone()));
            },
        );
    }
    group.finish();
}

//iterating

fn iterate_all_elements_vec(src: &SortedVecWithStrKey<TestStruct>) {
    for item in src.iter() {
        let a = item.id.clone();
    }
}

fn iterate_all_elements_hash_map(src: &HashMap<String, TestStruct>) {
    for (_, strc) in src.iter() {
        let a = strc.id.clone();
    }
}

pub fn iter_bench(c: &mut Criterion) {
    let start_init_data = 1_000;

    let test_data = [
        generate_init_data(start_init_data),
        generate_init_data(start_init_data * 2),
        generate_init_data(start_init_data * 4),
        generate_init_data(start_init_data * 6),
        generate_init_data(start_init_data * 10),
        generate_init_data(start_init_data * 20),
        generate_init_data(start_init_data * 40),
        generate_init_data(start_init_data * 60),
        generate_init_data(start_init_data * 80),
        generate_init_data(start_init_data * 100),
        // generate_init_data(start_init_data * 200),
        // generate_init_data(start_init_data * 400),
    ];

    let mut group = c.benchmark_group("sorted_vec_vs_hashmap-iter_bench");
    group.measurement_time(Duration::from_secs(10));
    for data in test_data.iter() {
        group.throughput(Throughput::Elements(data.1.len() as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("Vec {}", data.1.len())),
            &data.1.clone(),
            |b, src| {
                b.iter(|| iterate_all_elements_vec(src));
            },
        );

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("HashMap {}", data.1.len())),
            &data.0.clone(),
            |b, src| {
                b.iter(|| iterate_all_elements_hash_map(src));
            },
        );
    }
    group.finish();
}

criterion_group!(benches, iter_bench, inset_bench, search_bench);
criterion_main!(benches);
