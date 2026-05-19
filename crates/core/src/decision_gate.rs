//! Decision Gate rules have moved to the dedicated `arpagona-decision-gate` crate.
//!
//! This file is intentionally not exported by `crates/core/src/lib.rs`.
//! `crates/core` must remain the pure domain vocabulary crate.
//!
//! Use `arpagona_decision_gate::evaluate_proposed_action` and
//! `arpagona_decision_gate::audit_event_for_decision` from downstream crates.
