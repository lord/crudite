#[macro_use]
extern crate criterion;

use crudite::*;

use criterion::black_box;
use criterion::Criterion;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("character insert", |b| {
        let mut doc = Doc::new();
        doc.update(DocOp {
            timestamp: 1,
            edits: vec![
                tree::Edit::TextCreate { id: Id { num: 1 } },
                tree::Edit::MapInsert {
                    parent: ROOT_ID,
                    key: "my key".to_string(),
                    item: tree::Value::Collection(Id { num: 1 }),
                },
            ],
        });

        let mut i = 2;
        b.iter(|| {
            doc.update(DocOp {
                timestamp: i as u64,
                edits: vec![black_box(tree::Edit::TextInsert {
                    prev: Id { num: i - 1 },
                    id: Id { num: i },
                    character: if i % 2 == 0 { 'a' } else { 'b' },
                })],
            });
            i += 1;
        });
        black_box(doc);
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
