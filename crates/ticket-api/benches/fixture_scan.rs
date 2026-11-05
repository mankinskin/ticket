//! Criterion benchmark: scan throughput over the generated large fixture variant.
//!
//! Materializes the shared multi-store fixture with a benchmark-scale set of
//! generated tickets and measures full reindex (`scan(true)`) over the root
//! ticket store. Complements `graph_ops` by exercising the on-disk discovery
//! and indexing path against the canonical fixture. Also compares full and incremental
//! workspace-policy reconciliation on a warm fixture.

use criterion::{Criterion, criterion_group, criterion_main};
use memory_fixtures::materialize_fixture_with_generated_tickets;
use ticket_api::storage::store::TicketStore;

fn bench_fixture_scan(c: &mut Criterion) {
    let generated = 200usize;

    c.bench_function("fixture_scan_reindex_root_store", |b| {
        b.iter_batched(
            || {
                let fixture = materialize_fixture_with_generated_tickets(generated)
                    .expect("fixture should materialize");
                let store_root = fixture
                    .store_root("ticket-root")
                    .expect("ticket-root path")
                    .to_path_buf();
                let store = TicketStore::open_or_init(&store_root).expect("open_or_init");
                (fixture, store)
            },
            |(fixture, store)| {
                store
                    .scan(true)
                    .expect("scan should index generated tickets");
                drop(fixture);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_startup_policy(c: &mut Criterion) {
    let fixture = materialize_fixture_with_generated_tickets(200).expect("fixture");
    let root = fixture.store_root("ticket-root").expect("ticket-root");
    let workspace = TicketStore::resolve_workspace_store(
        root,
        memory_kernel::domain_store::StoreAccessMode::CreateOrOpen,
    )
    .expect("resolve workspace")
    .local_workspace;
    let store = TicketStore::open_or_init(root).expect("open fixture");
    store
        .reapply_workspace_policy(&workspace)
        .expect("warm indexes");

    let mut group = c.benchmark_group("fixture_startup_policy");
    group.bench_function("full_reindex", |b| {
        b.iter(|| {
            criterion::black_box(
                store
                    .reapply_workspace_policy(&workspace)
                    .expect("full scan"),
            );
        });
    });
    group.bench_function("incremental", |b| {
        b.iter(|| {
            criterion::black_box(
                store
                    .reconcile_workspace_policy(&workspace)
                    .expect("incremental scan"),
            );
        });
    });
    group.finish();
}

criterion_group!(benches, bench_fixture_scan, bench_startup_policy);
criterion_main!(benches);
