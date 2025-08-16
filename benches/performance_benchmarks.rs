use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Import the serdify library functions
use serdify::{from_str as custom_from_str, Result as CustomResult};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct BenchStruct {
    id: u64,
    name: String,
    email: String,
    age: u32,
    active: bool,
    score: f64,
    tags: Vec<String>,
    metadata: HashMap<String, String>,
}

fn generate_test_data(size: usize) -> String {
    let mut items = Vec::new();
    for i in 0..size {
        let item = BenchStruct {
            id: i as u64,
            name: format!("User{i}"),
            email: format!("user{i}@example.com"),
            age: 20 + (i % 50) as u32,
            active: i % 2 == 0,
            score: (i as f64) * 3.14,
            tags: vec![format!("tag{}", i), format!("category{}", i % 10)],
            metadata: {
                let mut map = HashMap::new();
                map.insert(format!("key{i}"), format!("value{i}"));
                map.insert("type".to_string(), "user".to_string());
                map
            },
        };
        items.push(serde_json::to_string(&item).unwrap());
    }
    format!("[{}]", items.join(","))
}

fn bench_small_json(c: &mut Criterion) {
    let json = r#"{"id": 1, "name": "John", "email": "john@example.com", "age": 30, "active": true, "score": 95.5, "tags": ["rust", "serde"], "metadata": {"role": "admin"}}"#;
    
    let mut group = c.benchmark_group("small_json");
    
    group.bench_function("serde_json", |b| {
        b.iter(|| {
            let result: std::result::Result<BenchStruct, _> = serde_json::from_str(black_box(json));
            black_box(result)
        })
    });
    
    group.bench_function("serdify", |b| {
        b.iter(|| {
            let result: CustomResult<BenchStruct> = custom_from_str(black_box(json));
            black_box(result)
        })
    });
    
    group.finish();
}

fn bench_medium_json(c: &mut Criterion) {
    let json = generate_test_data(100);
    
    let mut group = c.benchmark_group("medium_json");
    
    group.bench_function("serde_json", |b| {
        b.iter(|| {
            let result: std::result::Result<Vec<BenchStruct>, _> = serde_json::from_str(black_box(&json));
            black_box(result)
        })
    });
    
    group.bench_function("serdify", |b| {
        b.iter(|| {
            let result: CustomResult<Vec<BenchStruct>> = custom_from_str(black_box(&json));
            black_box(result)
        })
    });
    
    group.finish();
}

fn bench_large_json(c: &mut Criterion) {
    let json = generate_test_data(1000);
    
    let mut group = c.benchmark_group("large_json");
    group.sample_size(10); // Reduce sample size for large data
    
    group.bench_function("serde_json", |b| {
        b.iter(|| {
            let result: std::result::Result<Vec<BenchStruct>, _> = serde_json::from_str(black_box(&json));
            black_box(result)
        })
    });
    
    group.bench_function("serdify", |b| {
        b.iter(|| {
            let result: CustomResult<Vec<BenchStruct>> = custom_from_str(black_box(&json));
            black_box(result)
        })
    });
    
    group.finish();
}

fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling_comparison");
    
    for size in [10, 50, 100, 500].iter() {
        let json = generate_test_data(*size);
        
        group.bench_with_input(BenchmarkId::new("serde_json", size), &json, |b, json| {
            b.iter(|| {
                let result: std::result::Result<Vec<BenchStruct>, _> = serde_json::from_str(black_box(json));
                black_box(result)
            })
        });
        
        group.bench_with_input(BenchmarkId::new("serdify", size), &json, |b, json| {
            b.iter(|| {
                let result: CustomResult<Vec<BenchStruct>> = custom_from_str(black_box(json));
                black_box(result)
            })
        });
    }
    
    group.finish();
}

fn bench_primitive_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("primitive_types");
    
    let test_cases = vec![
        ("bool", "true"),
        ("i32", "42"),
        ("u64", "18446744073709551615"),
        ("f64", "3.141592653589793"),
        ("string", r#""hello world this is a longer string for testing""#),
    ];
    
    for (type_name, json) in test_cases {
        group.bench_with_input(BenchmarkId::new("serde_json", type_name), json, |b, json| {
            b.iter(|| {
                let result: std::result::Result<serde_json::Value, _> = serde_json::from_str(black_box(json));
                black_box(result)
            })
        });
        
        group.bench_with_input(BenchmarkId::new("serdify", type_name), json, |b, json| {
            b.iter(|| {
                let result: CustomResult<serde_json::Value> = custom_from_str(black_box(json));
                black_box(result)
            })
        });
    }
    
    group.finish();
}

fn bench_error_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_scenarios");
    
    // Test performance when errors occur
    let invalid_json = r#"{"id": "not_a_number", "name": 123, "active": "not_a_bool"}"#;
    
    group.bench_function("serde_json_errors", |b| {
        b.iter(|| {
            let result: std::result::Result<BenchStruct, _> = serde_json::from_str(black_box(invalid_json));
            black_box(result)
        })
    });
    
    group.bench_function("serdify_errors", |b| {
        b.iter(|| {
            let result: CustomResult<BenchStruct> = custom_from_str(black_box(invalid_json));
            black_box(result)
        })
    });
    
    group.finish();
}

fn bench_nested_structures(c: &mut Criterion) {
    let mut group = c.benchmark_group("nested_structures");
    
    let nested_json = r#"{
        "level1": {
            "level2": {
                "level3": {
                    "level4": {
                        "level5": {
                            "deep_value": "very deep",
                            "array": [1, 2, 3, 4, 5],
                            "map": {"a": 1, "b": 2, "c": 3}
                        }
                    }
                }
            }
        }
    }"#;
    
    group.bench_function("serde_json_nested", |b| {
        b.iter(|| {
            let result: std::result::Result<serde_json::Value, _> = serde_json::from_str(black_box(nested_json));
            black_box(result)
        })
    });
    
    group.bench_function("serdify_nested", |b| {
        b.iter(|| {
            let result: CustomResult<serde_json::Value> = custom_from_str(black_box(nested_json));
            black_box(result)
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_small_json,
    bench_medium_json,
    bench_large_json,
    bench_scaling,
    bench_primitive_types,
    bench_error_scenarios,
    bench_nested_structures
);
criterion_main!(benches);
