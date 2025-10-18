use criterion::{criterion_group, criterion_main, Criterion};
use rayon::prelude::*;

// --- Old regex version for comparison ---
fn old_add_commas(whole: &str) -> String {
    let re = regex::Regex::new(r"^(-?\d+)(\d{3})").unwrap();
    let mut w = whole.to_string();
    loop {
        let new = re.replace(&w, "$1,$2").to_string();
        if new == w {
            break;
        }
        w = new;
    }
    w
}

// --- New optimized version ---
fn add_commas(whole: &str) -> String {
    let chars: Vec<char> = whole.chars().collect();
    let mut result = String::new();
    let mut count = 0;
    for &c in chars.iter().rev() {
        if count == 3 {
            result.push(',');
            count = 0;
        }
        result.push(c);
        count += 1;
    }
    result.chars().rev().collect()
}

// Helper: generate a large dataset of numeric strings
fn generate_numbers(n: usize) -> Vec<String> {
    (0..n)
        .map(|i| format!("{}", i * 12345)) // multiply to get larger numbers
        .collect()
}

fn benchmark_old(c: &mut Criterion) {
    let data = generate_numbers(100_000);
    c.bench_function("old regex loop", |b| {
        b.iter(|| {
            let _: Vec<String> = data.iter().map(|s| old_add_commas(s)).collect();
        })
    });
}

fn benchmark_new(c: &mut Criterion) {
    let data = generate_numbers(100_000);
    c.bench_function("new optimized + rayon", |b| {
        b.iter(|| {
            let _: Vec<String> = data
                .par_iter()
                .map(|s| add_commas(s))
                .collect();
        })
    });
}

criterion_group!(benches, benchmark_old, benchmark_new);
criterion_main!(benches);
