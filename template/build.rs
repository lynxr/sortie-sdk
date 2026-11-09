// Типизированные символы сценария (groups::ARMOR, zones::TARGET, ...) из scenario.lock.json.
fn main() {
    sortie_build::scenario("my_mission", "scenario.lock.json").emit();
}
