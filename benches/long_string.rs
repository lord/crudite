#[macro_use]
extern crate criterion;

use crudite::*;

use criterion::Criterion;
use criterion::black_box;

fn character_insert_bench(n: usize) -> Doc {
    let mut doc = Doc::new();
    doc.update(DocOp {
        timestamp: 1,
        edits: vec![
            tree::Edit::TextCreate{
                id: Id {num: 1},
            },
            tree::Edit::MapInsert{
                parent: ROOT_ID,
                key: "my key".to_string(),
                item: tree::Value::Collection(Id {num: 1}),
            },
        ],
    });

    for i in 2..n {
        doc.update(DocOp {
            timestamp: i as u64,
            edits: vec![
                black_box(tree::Edit::TextInsert {
                    prev: Id {num: i-1},
                    id: Id {num: i},
                    character: if i % 2 == 0 {'a'} else {'b'},
                }),
            ],
        });
    }

    doc
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("character insert x 1", |b| b.iter(|| character_insert_bench(1)));
    c.bench_function("character insert x 10", |b| b.iter(|| character_insert_bench(10)));
    c.bench_function("character insert x 100", |b| b.iter(|| character_insert_bench(100)));
    c.bench_function("character insert x 1_000", |b| b.iter(|| character_insert_bench(1_000)));
    c.bench_function("character insert x 10_000", |b| b.iter(|| character_insert_bench(10_000)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
