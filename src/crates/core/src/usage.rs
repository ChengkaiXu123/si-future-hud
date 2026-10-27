//! Token counts as the ledger keeps them, and the usage projection a snapshot
//! is drawn from until a source write changes it.
use crate::Usage;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

/// Usage changes and display-clock changes have separate lifetimes, as in
/// UsageLedger/UsageStore: only source writes invalidate this numeric projection.
pub(crate) struct UsageProjection {
    pub(crate) selected: Vec<PathBuf>,
    pub(crate) totals: Usage,
    pub(crate) timeline: Vec<Value>,
    pub(crate) titles: HashMap<String, String>,
    pub(crate) context_percents: HashMap<String, f64>,
    pub(crate) observed_providers: HashSet<String>,
    pub(crate) counts: HashMap<String, usize>,
}
