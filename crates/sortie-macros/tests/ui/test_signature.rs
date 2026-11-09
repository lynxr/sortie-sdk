struct Harness;

#[sortie::test]
fn not_async(_h: Harness) {}

#[sortie::test]
async fn no_harness() {}

#[sortie::test]
async fn returns_a_value(_h: Harness) -> u32 {
    1
}

#[sortie::test]
#[test]
async fn twice(_h: Harness) {}

fn main() {}
