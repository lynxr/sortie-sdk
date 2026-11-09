struct Harness;

#[sortie::test(path = "fixtures/op_north.manifest.pb")]
async fn unknown_argument(_h: Harness) {}

#[sortie::test(manifest = 42)]
async fn not_a_string(_h: Harness) {}

#[sortie::test(manifest = "fixtures/missing.manifest.pb")]
async fn missing_file(_h: Harness) {}

#[sortie::test(manifest = "scenario.miz")]
async fn not_a_manifest(_h: Harness) {}

fn main() {}
