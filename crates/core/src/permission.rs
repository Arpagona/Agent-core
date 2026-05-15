use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    ReadMemory,
    WriteMemory,
    ReadDocument,
    WriteDocument,
    ProposeToolUse,
    SimulateEmail,
    ManageTask,
    ManagePolicy,
}
