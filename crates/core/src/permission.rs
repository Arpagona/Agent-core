use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    ReadMemory,
    ReadTasks,
    ReadProposedActions,
    ReadDecisions,
    ReadAudit,
    ReadStatus,
    WriteMemory,
    ReadDocument,
    WriteDocument,
    ProposeToolUse,
    SimulateEmail,
    ManageTask,
    ManagePolicy,
}
