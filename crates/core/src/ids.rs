use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

id_type!(AgentId);
id_type!(AuditEventId);
id_type!(DecisionId);
id_type!(EpisodeId);
id_type!(FactId);
id_type!(FailureInsightId);
id_type!(GoalId);
id_type!(HolographicPatternId);
id_type!(HolographicTraceId);
id_type!(ObservationId);
id_type!(PolicyId);
id_type!(ProposedActionId);
id_type!(SourceId);
id_type!(TaskId);
id_type!(ToolExecutionId);
id_type!(ToolId);
id_type!(WorkspaceId);
