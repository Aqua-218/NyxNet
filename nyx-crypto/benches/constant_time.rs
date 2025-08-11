use criterion::{criterion_group, criterion_main, Criterion, black_box};
use x25519_dalek::{EphemeralSecret, PublicKey};
// Use the 0.6 compatibility crate for OsRng to satisfy EphemeralSecret::random_from_rng bounds
use rand_core_06::OsRng;

fn diffie_hellman_bench(c: &mut Criterion) {
    let secret = EphemeralSecret::random_from_rng(OsRng);
    let public = PublicKey::from(&secret);

    c.bench_function("x25519_dh", |b| {
        b.iter(|| {
            let _ = secret.diffie_hellman(black_box(&public));
        })
    });
}

criterion_group!(benches, diffie_hellman_bench);
criterion_main!(benches); 