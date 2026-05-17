DEFINE TABLE fact SCHEMALESS;
DEFINE FIELD data ON TABLE fact TYPE object;
DEFINE FIELD entity_type ON TABLE fact TYPE string;
DEFINE FIELD entity_id ON TABLE fact TYPE string;
DEFINE FIELD status ON TABLE fact TYPE string;
DEFINE FIELD created_at ON TABLE fact TYPE string;
DEFINE INDEX fact_entity ON TABLE fact COLUMNS entity_type, entity_id;

DEFINE TABLE source SCHEMALESS;
DEFINE FIELD data ON TABLE source TYPE object;

DEFINE TABLE episode SCHEMALESS;
DEFINE FIELD data ON TABLE episode TYPE object;
DEFINE FIELD workspace_id ON TABLE episode TYPE string;
DEFINE INDEX episode_workspace ON TABLE episode COLUMNS workspace_id;

DEFINE TABLE observation SCHEMALESS;
DEFINE FIELD data ON TABLE observation TYPE object;
DEFINE FIELD episode_id ON TABLE observation TYPE string;
DEFINE INDEX observation_episode ON TABLE observation COLUMNS episode_id;

DEFINE TABLE audit_event SCHEMALESS;
DEFINE FIELD data ON TABLE audit_event TYPE object;
DEFINE FIELD workspace_id ON TABLE audit_event TYPE option<string>;
DEFINE FIELD created_at ON TABLE audit_event TYPE string;
DEFINE INDEX audit_event_workspace ON TABLE audit_event COLUMNS workspace_id;

DEFINE TABLE decision SCHEMALESS;
DEFINE FIELD data ON TABLE decision TYPE object;
DEFINE FIELD proposed_action_id ON TABLE decision TYPE string;
DEFINE INDEX decision_proposed_action ON TABLE decision COLUMNS proposed_action_id;

DEFINE TABLE proposed_action SCHEMALESS;
DEFINE FIELD data ON TABLE proposed_action TYPE object;
DEFINE FIELD workspace_id ON TABLE proposed_action TYPE string;
DEFINE INDEX proposed_action_workspace ON TABLE proposed_action COLUMNS workspace_id;
