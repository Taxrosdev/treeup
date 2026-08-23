use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use std::{fs, io, path::Path, sync::Arc};
use temp_dir::TempDir;
use tokio::runtime::Builder;
use treeup::object::{Deployable, cas::BasicFS};

/// Helper function to create a heap of garbage that can be used as an example tree
fn create_fake_tree(path: &Path, complexity: usize) -> io::Result<()> {
    // Don't just infinitely run...
    if complexity == 0 {
        return Ok(());
    }

    for i in 0..=complexity {
        let path = path.join(i.to_string());
        fs::create_dir(&path).unwrap();
        create_fake_tree(&path, complexity / 2).unwrap();
    }

    for i in 0..=complexity {
        let path = path.join(format!("{i}.file"));
        fs::write(&path, "asdasd").unwrap();
    }

    Ok(())
}

fn e2e(c: &mut Criterion) {
    let runtime = Builder::new_multi_thread()
        .worker_threads(4)
        .build()
        .unwrap();

    // Tree::create
    let mut group = c.benchmark_group("treeup_create");
    for (name, complexity) in [("small", 2), ("moderate", 8), ("large", 12)] {
        let source_dir = TempDir::new().unwrap();
        create_fake_tree(source_dir.path(), complexity).unwrap();
        group.bench_with_input(BenchmarkId::from_parameter(name), &complexity, |b, &_| {
            b.to_async(&runtime).iter_batched(
                || {
                    tokio::task::block_in_place(|| {
                        runtime.block_on(async {
                            let temp_dir = TempDir::new().unwrap();
                            let objects_path = temp_dir.child("objects");
                            let cas = Arc::new(BasicFS::create(objects_path).await.unwrap());
                            let blobs_path = temp_dir.child("blobs");

                            (cas, blobs_path, &source_dir, temp_dir)
                        })
                    })
                },
                |(cas, blobs_path, source_dir, _temp_dir)| async move {
                    treeup::Tree::create(cas, &blobs_path, source_dir.path())
                        .await
                        .unwrap();
                    // Manually drop to force _temp_dir to be held on.
                    // Compiler is dropping it despite it being in scope?
                    drop(_temp_dir)
                },
                BatchSize::SmallInput,
            );
        });
    }
    // FS Operations are very flimsy.
    group.noise_threshold(0.05);
    group.finish();

    // Tree::deploy
    let mut group = c.benchmark_group("treeup_deploy");
    for (name, complexity) in [("small", 2), ("moderate", 8), ("large", 12)] {
        let source_dir = TempDir::new().unwrap();
        create_fake_tree(source_dir.path(), complexity).unwrap();
        group.bench_with_input(BenchmarkId::from_parameter(name), &complexity, |b, &_| {
            b.to_async(&runtime).iter_batched(
                || {
                    tokio::task::block_in_place(|| {
                        runtime.block_on(async {
                            let temp_dir = TempDir::new().unwrap();
                            let objects_path = temp_dir.child("objects");
                            let cas = Arc::new(BasicFS::create(objects_path).await.unwrap());
                            let blobs_path = temp_dir.child("blobs");

                            let tree =
                                treeup::Tree::create(cas.clone(), &blobs_path, source_dir.path())
                                    .await
                                    .unwrap();
                            let deploy_dir = TempDir::new().unwrap();

                            (tree, cas, blobs_path, deploy_dir, temp_dir)
                        })
                    })
                },
                |(tree, cas, blobs_path, deploy_dir, _temp_dir)| async move {
                    tree.deploy_recursive(cas, &blobs_path, deploy_dir.path())
                        .await
                        .unwrap();
                    // Manually force _temp_dir to be held on.
                    // Compiler is dropping it despite it being in scope?
                    let _unused = _temp_dir.path();
                },
                BatchSize::LargeInput,
            );
        });
    }
    // FS Operations are very flimsy.
    group.noise_threshold(0.05);
    group.significance_level(0.05);
    group.finish();
}

criterion_group!(benches, e2e);
criterion_main!(benches);
