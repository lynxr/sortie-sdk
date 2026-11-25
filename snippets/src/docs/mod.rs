//! Examples of the client docs (`sdk/docs`). Each module holds the regions of one page or
//! topic between `// snippet:start <id>` and `// snippet:end <id>`; `cargo xtask docs-check`
//! compares them with the code blocks of the pages.
//!
//! The scenario is `python/docs_scenario.py` (its lock is `scenario/scenario.lock.json`): a
//! superset of the mission template's, so the template's mission compiles here unchanged.

sortie::include_scenario!(my_mission);

pub mod quickstart;
pub mod sdk_airbases;
pub mod sdk_caps;
pub mod sdk_commands;
pub mod sdk_crate;
pub mod sdk_dcs;
pub mod sdk_errors;
pub mod sdk_events;
pub mod sdk_map;
pub mod sdk_menu_tree;
pub mod sdk_menus;
pub mod sdk_objects;
pub mod sdk_state;
pub mod sdk_wait;
pub mod sdk_warehouses;
pub mod testing;
pub mod web_api;
