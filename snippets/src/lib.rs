//! Code excerpts for swiftide.rs.
//!
//! Pages include these with `::snippet{src="@snippets/src/<file>.rs" anchor="<name>"}`. Each
//! excerpt sits between `// ANCHOR: <name>` and `// ANCHOR_END: <name>` markers inside a function
//! that compiles, so every line a reader sees has been type-checked against the pinned swiftide.
//! Modules mirror the site's sections.

pub mod agents;
pub mod cookbook;
pub mod indexing;
pub mod integrations;
pub mod models;
pub mod observability;
pub mod querying;
pub mod reference;
pub mod start;
pub mod tasks;
