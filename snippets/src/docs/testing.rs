//! What a logic crate can test without DCS in SDK 0.1: the scenario's symbols and geometry, and
//! its own pure functions.

#[cfg(test)]
mod tests {
    use sortie::prelude::*;

    use crate::docs::my_mission::zones;

    // snippet:start testing-unit
    #[test]
    fn the_target_lies_on_the_front_line() {
        assert!(zones::FRONT.contains(zones::TARGET.center()));
        let gap = zones::TARGET.center().distance(zones::PATROL.center());
        assert!(gap < km(1).meters(), "the patrol is centred on the target");
    }
    // snippet:end testing-unit
}
