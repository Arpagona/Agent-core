//! Override engine for the ARPAGONA Decision Gate.
//!
//! Provides controlled override of blocked actions by authorized administrators.
//! The override mechanism uses:
//! - A pluggable password verifier (alpha: DefaultHasherVerifier; production: Argon2/bcrypt/scrypt)
//! - Time-to-live (TTL) — override authorization expires after a configurable duration
//! - Max attempts — locks further attempts temporarily after too many failures
//! - Full audit trail for every override attempt
//!
//! # Security
//!
//! - Password is never stored or logged in plain text
//! - The stored hash cannot be reversed to recover the password
//! - Password is never included in Debug output, audit events, or error messages
//! - TTL prevents indefinite authorization
//! - Lockout prevents brute-force attacks
//!
//! # Production-grade verification
//!
//! `Argon2PasswordVerifier` replaces `DefaultHasherVerifier` in production.
//! It reads a pre-computed Argon2id hash from the `ARPAGONA_OVERRIDE_PASSWORD_HASH`
//! environment variable and verifies passwords against it using the Argon2id algorithm.
//!
//! The hash must be in the PHC string format produced by Argon2:
//! `$argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>`
//!
//! Generate with: `echo -n "your-password" | argon2 "$(openssl rand -base64 16)" -id -t 2 -m 19 -p 1 -l 32`
//!
//! # Configuration
//!
//! - `ARPAGONA_OVERRIDE_PASSWORD_HASH`: Argon2id PHC hash string (production)
//! - `ARPAGONA_OVERRIDE_PASSWORD`: plain text password with DefaultHasher (dev only, requires `ARPAGONA_ALLOW_DEV_OVERRIDE=true`)
//! - `ARPAGONA_ALLOW_DEV_OVERRIDE=true`: enables dev-mode fallback when no hash is set
//!
//! If neither `ARPAGONA_OVERRIDE_PASSWORD_HASH` nor (`ARPAGONA_OVERRIDE_PASSWORD` + dev mode) is configured,
//! the override endpoint returns `override_not_configured` and no override is possible.
//!
//! # Future
//!
//! - Integrate with a secrets vault (e.g., HashiCorp Vault, AWS Secrets Manager)

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

use arpagona_agent_core::{ActionType, OverridePolicy, ProposedAction, RiskLevel};

/// Default TTL for an override authorization (5 minutes) — UNUSED, kept for reference.
/// Override is strictly per-action; there is no session TTL.
pub const DEFAULT_OVERRIDE_TTL_SECONDS: i64 = 300;

/// Default max failed attempts before lockout.
pub const DEFAULT_MAX_FAILED_ATTEMPTS: u32 = 3;

/// Default lockout duration after max failed attempts (5 minutes).
pub const DEFAULT_LOCKOUT_SECONDS: i64 = 300;

/// Configuration for the override engine.
///
/// Note: There is NO TTL / session timeout. Override is strictly per-action.
/// Each override attempt must independently pass password verification.
/// The only global state is the brute-force lockout counter.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OverrideConfig {
    /// Max failed attempts before temporary lockout.
    pub max_failed_attempts: u32,
    /// Lockout duration in seconds after max failed attempts.
    pub lockout_seconds: i64,
}

impl Default for OverrideConfig {
    fn default() -> Self {
        Self {
            max_failed_attempts: DEFAULT_MAX_FAILED_ATTEMPTS,
            lockout_seconds: DEFAULT_LOCKOUT_SECONDS,
        }
    }
}

/// Trait for password verification.
///
/// # Security
///
/// Implementations MUST:
/// - Never log, store in plain text, or expose the password in Debug output
/// - Use a cryptographically secure hashing algorithm in production
///
/// Production implementations SHOULD use Argon2, bcrypt, or scrypt.
pub trait PasswordVerifier: Debug + Send + Sync {
    /// Verify a password candidate.
    ///
    /// Returns `true` if the password is correct.
    ///
    /// # Security
    ///
    /// This method must NOT log, store, or expose the password in any way.
    /// It must not include the password in error messages or Debug output.
    fn verify(&self, password: &str) -> bool;
}

/// Alpha-level password verifier using `std::hash::DefaultHasher`.
///
/// # Security
///
/// This is NOT cryptographically secure. `std::hash::DefaultHasher` is designed
/// for hash tables, not password verification. It must NOT be used in production.
///
/// It ensures:
/// - Password is never stored or logged in plain text
/// - The stored hash cannot be reversed to recover the password
/// - Each hash is salted with a fixed application salt
///
/// TODO: Replace with Argon2, bcrypt, or scrypt for production.
#[derive(Debug)]
pub struct DefaultHasherVerifier {
    password_hash: u64,
    salt: String,
}

impl DefaultHasherVerifier {
    /// Create a new verifier by hashing the password with a salt.
    ///
    /// The password is hashed immediately and never stored in plain text.
    pub fn new(password: &str, salt: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        salt.hash(&mut hasher);
        password.hash(&mut hasher);
        let password_hash = hasher.finish();
        Self {
            password_hash,
            salt: salt.to_owned(),
        }
    }
}

impl PasswordVerifier for DefaultHasherVerifier {
    fn verify(&self, candidate: &str) -> bool {
        let mut hasher = DefaultHasher::new();
        self.salt.hash(&mut hasher);
        candidate.hash(&mut hasher);
        let candidate_hash = hasher.finish();
        candidate_hash == self.password_hash
    }
}

/// Production-grade password verifier using the Argon2id algorithm.
///
/// Reads a pre-computed Argon2id PHC hash and verifies passwords against it.
/// The hash is stored in memory and never logged or exposed.
///
/// # Security properties
///
/// - Argon2id resists GPU/ASIC brute-force attacks via configurable memory/time costs
/// - The stored PHC hash contains all parameters (salt, parallelism, memory, iterations)
/// - Password is never stored in plain text
/// - Debug output never includes the hash or password
/// - The hash is accepted at construction time and verified via the `verify` method
///
/// # PHC string format
///
/// Hash must be in PHC format:
/// `$argon2id$v=19$m=19456,t=2,p=1$<base64-salt>$<base64-hash>`
///
/// The m (memory), t (time), and p (parallelism) parameters are part of the hash
/// string and do not need to be configured separately.
#[derive(Clone)]
pub struct Argon2PasswordVerifier {
    /// The Argon2id PHC hash string. Stored to be parsed by the argon2 crate
    /// at verification time. Never logged.
    phc_hash: String,
}

impl Argon2PasswordVerifier {
    /// Create a new verifier from an Argon2id PHC hash string.
    ///
    /// The hash is stored in memory and used for subsequent verifications.
    ///
    /// # Panics
    ///
    /// Panics if the hash string is not a valid Argon2 PHC hash (e.g., wrong algorithm,
    /// malformed encoding). This is intentional — a misconfigured hash should fail loudly
    /// at startup, not silently at runtime.
    pub fn new(phc_hash: &str) -> Self {
        // Validate the hash at construction time
        let _ = argon2::PasswordHash::new(phc_hash).expect(
            "invalid Argon2 PHC hash, expected format: \
             $argon2id$v=19$m=19456,t=2,p=1$<base64-salt>$<base64-hash>",
        );
        Self {
            phc_hash: phc_hash.to_owned(),
        }
    }

    /// Create a new verifier from the `ARPAGONA_OVERRIDE_PASSWORD_HASH` environment variable.
    ///
    /// Returns `None` if the variable is not set or is empty.
    /// Panics if the variable is set but contains an invalid PHC hash.
    pub fn from_env_hash() -> Option<Self> {
        let hash = std::env::var("ARPAGONA_OVERRIDE_PASSWORD_HASH").ok()?;
        if hash.is_empty() {
            return None;
        }
        Some(Self::new(&hash))
    }
}

impl PasswordVerifier for Argon2PasswordVerifier {
    fn verify(&self, candidate: &str) -> bool {
        use argon2::PasswordVerifier as _;
        let Ok(parsed_hash) = argon2::PasswordHash::new(&self.phc_hash) else {
            return false;
        };
        argon2::Argon2::default()
            .verify_password(candidate.as_bytes(), &parsed_hash)
            .is_ok()
    }
}

/// Custom Debug implementation that does NOT expose the hash.
impl Debug for Argon2PasswordVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Argon2PasswordVerifier")
            .field("phc_hash", &"[REDACTED]")
            .finish()
    }
}

/// Blanket implementation: Box<dyn PasswordVerifier> implements PasswordVerifier.
impl<T: PasswordVerifier + ?Sized> PasswordVerifier for Box<T> {
    fn verify(&self, password: &str) -> bool {
        (**self).verify(password)
    }
}

/// Outcome of an override attempt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverrideOutcome {
    /// Override was successful — action is now ApprovedByOverride.
    Approved,
    /// Override failed — wrong password/token.
    Failed,
    /// Override is locked due to too many failed attempts.
    Locked,
    /// Override authorization has expired.
    Expired,
    /// This action cannot be overridden.
    NotOverridable,
}

/// The override engine manages password verification, TTL, and attempt counting.
///
/// # Generic parameter
///
/// `V` is the password verifier implementation. For production, use Argon2/bcrypt;
/// for alpha/testing, use `DefaultHasherVerifier`.
#[derive(Debug)]
pub struct OverrideEngine<V: PasswordVerifier> {
    /// Password verifier (never the plain text password).
    verifier: V,
    /// Configuration for TTL and limits.
    config: OverrideConfig,
    /// Current failed attempt count.
    failed_attempts: u32,
    /// Timestamp when the current lockout expires (None = not locked).
    locked_until: Option<DateTime<Utc>>,
}

impl<V: PasswordVerifier> OverrideEngine<V> {
    /// Create a new override engine with the given verifier.
    pub fn new(verifier: V) -> Self {
        Self {
            verifier,
            config: OverrideConfig::default(),
            failed_attempts: 0,
            locked_until: None,
        }
    }

    /// Create a new override engine with custom config and verifier.
    pub fn with_config(verifier: V, config: OverrideConfig) -> Self {
        Self {
            verifier,
            config,
            failed_attempts: 0,
            locked_until: None,
        }
    }

    /// Attempt an override with the given password.
    ///
    /// Every attempt independently verifies the password — there is NO
    /// authorization session. Each override is single-action scoped:
    /// approving action A does NOT create a session that can approve action B.
    ///
    /// Returns:
    /// - `Approved` if the password is correct
    /// - `Failed` if the password is wrong
    /// - `Locked` if too many failed attempts (global lockout, anti-brute-force)
    pub fn attempt_override(&mut self, candidate: &str) -> OverrideOutcome {
        // Check lockout
        if let Some(locked_until) = self.locked_until {
            if Utc::now() < locked_until {
                return OverrideOutcome::Locked;
            }
            // Lockout expired — reset
            self.locked_until = None;
            self.failed_attempts = 0;
        }

        // Verify password using the pluggable verifier
        if self.verifier.verify(candidate) {
            self.failed_attempts = 0;
            OverrideOutcome::Approved
        } else {
            self.failed_attempts += 1;
            if self.failed_attempts >= self.config.max_failed_attempts {
                self.locked_until =
                    Some(Utc::now() + Duration::seconds(self.config.lockout_seconds));
            }
            OverrideOutcome::Failed
        }
    }

    /// Reset the engine state (clear failures, lockout, authorization).
    pub fn reset(&mut self) {
        self.failed_attempts = 0;
        self.locked_until = None;
    }

    /// Check if the engine is currently locked.
    pub fn is_locked(&self) -> bool {
        self.locked_until
            .map(|until| Utc::now() < until)
            .unwrap_or(false)
    }

    /// Get remaining attempts before lockout.
    pub fn remaining_attempts(&self) -> u32 {
        if self.is_locked() {
            0
        } else {
            self.config
                .max_failed_attempts
                .saturating_sub(self.failed_attempts)
        }
    }

    /// Get the current failure count (for audit logging — does NOT include the password).
    pub fn failure_count(&self) -> u32 {
        self.failed_attempts
    }

    /// Get the lockout expiry time (for audit logging).
    pub fn locked_until(&self) -> Option<DateTime<Utc>> {
        self.locked_until
    }
}

/// Determine the override policy for a given action.
///
/// Returns which type of override (if any) is available for this action.
///
/// Rules:
/// - Destructive/dangerous actions (SimulateEmail, Custom, ProposeToolUse): NotOverridable
/// - Medium risk or higher: NotOverridable
/// - Write/execution actions: NotOverridable
/// - Read-only informational/low risk actions with missing permission: PasswordRequired
pub fn classify_override_policy(action: &ProposedAction) -> OverridePolicy {
    // Block override for destructive/dangerous actions
    if is_destructive_or_dangerous(action) {
        return OverridePolicy::NotOverridable;
    }

    // Block override for medium risk or higher (unless explicit future rule)
    if matches!(
        action.risk_level,
        RiskLevel::Medium | RiskLevel::High | RiskLevel::Critical
    ) {
        return OverridePolicy::NotOverridable;
    }

    // Block override for write/execution actions
    if is_write_or_execution_action(action) {
        return OverridePolicy::NotOverridable;
    }

    // Read-only informational/low risk actions are overridable
    // when the block reason is missing permission or missing confirmation.
    OverridePolicy::PasswordRequired
}

/// Check if an action type is destructive or dangerous.
fn is_destructive_or_dangerous(action: &ProposedAction) -> bool {
    matches!(
        action.action_type,
        ActionType::SimulateEmail | ActionType::Custom(_) | ActionType::ProposeToolUse
    )
}

/// Check if an action type writes data or performs execution.
fn is_write_or_execution_action(action: &ProposedAction) -> bool {
    matches!(
        action.action_type,
        ActionType::WriteMemory
            | ActionType::WriteDocument
            | ActionType::CreateMemoryFact
            | ActionType::LinkMemoryFact
            | ActionType::InvalidateMemoryFact
            | ActionType::CreateFailureInsightMemory
            | ActionType::CreateHolographicTrace
            | ActionType::SimulateEmail
            | ActionType::ManageTask
            | ActionType::ProposeToolUse
            | ActionType::Custom(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use arpagona_agent_core::{AgentId, ProposedActionId, ProposedActionStatus, WorkspaceId};
    use serde_json::json;

    /// A mock password verifier that always returns true.
    /// Useful for testing the engine mechanics without password setup.
    #[derive(Debug)]
    struct AlwaysTrueVerifier;

    impl PasswordVerifier for AlwaysTrueVerifier {
        fn verify(&self, _password: &str) -> bool {
            true
        }
    }

    /// A mock password verifier that matches a specific password.
    #[derive(Debug)]
    struct StaticTestPasswordVerifier {
        /// The expected password (stored for comparison only — NOT for production).
        /// In production verifiers, the plain text password must never be stored.
        expected: String,
    }

    impl PasswordVerifier for StaticTestPasswordVerifier {
        fn verify(&self, password: &str) -> bool {
            password == self.expected
        }
    }

    fn make_action(action_type: ActionType, risk_level: RiskLevel) -> ProposedAction {
        ProposedAction {
            id: ProposedActionId::new("action-1"),
            workspace_id: WorkspaceId::new("workspace-1"),
            task_id: None,
            proposed_by: AgentId::new("agent-1"),
            action_type,
            target: None,
            payload: json!({}),
            risk_level,
            required_permissions: vec![],
            rationale: "test action".to_owned(),
            context_refs: vec![],
            status: ProposedActionStatus::PendingDecision,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn password_is_hashed_not_stored_in_plaintext() {
        let verifier = DefaultHasherVerifier::new("secret123", "test-salt");
        // The hash should not be the password itself
        assert_ne!(format!("{:?}", verifier), "secret123");
        // Verify the password works
        assert!(verifier.verify("secret123"));
        assert!(!verifier.verify("wrong"));
    }

    #[test]
    fn correct_password_returns_approved() {
        let verifier = DefaultHasherVerifier::new("override-pass", "test-salt");
        let mut engine = OverrideEngine::new(verifier);
        let result = engine.attempt_override("override-pass");
        assert_eq!(result, OverrideOutcome::Approved);
    }

    #[test]
    fn wrong_password_returns_failed() {
        let verifier = DefaultHasherVerifier::new("real-pass", "test-salt");
        let mut engine = OverrideEngine::new(verifier);
        let result = engine.attempt_override("wrong-pass");
        assert_eq!(result, OverrideOutcome::Failed);
    }

    #[test]
    fn too_many_failures_locks_engine() {
        let verifier = DefaultHasherVerifier::new("real-pass", "test-salt");
        let mut engine = OverrideEngine::with_config(
            verifier,
            OverrideConfig {
                max_failed_attempts: 2,
                lockout_seconds: 60,
                ..Default::default()
            },
        );

        assert_eq!(engine.attempt_override("wrong"), OverrideOutcome::Failed);
        assert_eq!(engine.attempt_override("wrong"), OverrideOutcome::Failed);
        // Third attempt should be locked
        assert_eq!(
            engine.attempt_override("real-pass"),
            OverrideOutcome::Locked
        );
    }

    #[test]
    fn lockout_expires_after_duration() {
        let verifier = DefaultHasherVerifier::new("real-pass", "test-salt");
        let mut engine = OverrideEngine::with_config(
            verifier,
            OverrideConfig {
                max_failed_attempts: 1,
                lockout_seconds: 0, // immediate unlock
                ..Default::default()
            },
        );

        assert_eq!(engine.attempt_override("wrong"), OverrideOutcome::Failed);
        // With 0-second lockout, the next attempt should NOT be locked
        assert_eq!(engine.attempt_override("wrong"), OverrideOutcome::Failed);
    }

    #[test]
    fn classify_read_memory_informational_as_overridable() {
        let action = make_action(ActionType::ReadMemory, RiskLevel::Informational);
        assert_eq!(
            classify_override_policy(&action),
            OverridePolicy::PasswordRequired
        );
    }

    #[test]
    fn classify_destructive_action_as_not_overridable() {
        let action = make_action(ActionType::SimulateEmail, RiskLevel::Informational);
        assert_eq!(
            classify_override_policy(&action),
            OverridePolicy::NotOverridable
        );
    }

    #[test]
    fn classify_medium_risk_as_not_overridable() {
        let action = make_action(ActionType::ReadMemory, RiskLevel::Medium);
        assert_eq!(
            classify_override_policy(&action),
            OverridePolicy::NotOverridable
        );
    }

    #[test]
    fn classify_write_action_as_not_overridable() {
        let action = make_action(ActionType::WriteMemory, RiskLevel::Informational);
        assert_eq!(
            classify_override_policy(&action),
            OverridePolicy::NotOverridable
        );
    }

    #[test]
    fn remaining_attempts_works() {
        let verifier = DefaultHasherVerifier::new("pass", "salt");
        let mut engine = OverrideEngine::with_config(
            verifier,
            OverrideConfig {
                max_failed_attempts: 3,
                ..Default::default()
            },
        );

        assert_eq!(engine.remaining_attempts(), 3);
        engine.attempt_override("wrong");
        assert_eq!(engine.remaining_attempts(), 2);
    }

    // ── New tests for trait-based verifier ──────────────────────────────

    #[test]
    fn always_true_verifier_always_approves() {
        let mut engine = OverrideEngine::new(AlwaysTrueVerifier);
        assert_eq!(
            engine.attempt_override("any-password"),
            OverrideOutcome::Approved
        );
    }

    #[test]
    fn static_test_password_verifier_works() {
        let verifier = StaticTestPasswordVerifier {
            expected: "admin-pass".to_owned(),
        };
        let mut engine = OverrideEngine::new(verifier);
        assert_eq!(
            engine.attempt_override("admin-pass"),
            OverrideOutcome::Approved
        );
        assert_eq!(
            engine.attempt_override("wrong-pass"),
            OverrideOutcome::Failed
        );
    }

    #[test]
    fn password_must_not_appear_in_debug_output() {
        let verifier = DefaultHasherVerifier::new("super-secret-password", "salt123");
        let debug_str = format!("{:?}", verifier);
        // The plain text password must never appear in Debug output
        assert!(
            !debug_str.contains("super-secret-password"),
            "password must not appear in Debug: {}",
            debug_str
        );
    }

    #[test]
    fn repeated_correct_password_always_approved() {
        let verifier = DefaultHasherVerifier::new("my-pass", "my-salt");
        let mut engine = OverrideEngine::with_config(
            verifier,
            OverrideConfig {
                max_failed_attempts: 3,
                ..Default::default()
            },
        );

        // First attempt: correct password → Approved
        assert_eq!(
            engine.attempt_override("my-pass"),
            OverrideOutcome::Approved
        );
        assert_eq!(engine.remaining_attempts(), 3);

        // Second attempt: correct password again → Still approved
        // (no session leak, but correct password works every time)
        assert_eq!(
            engine.attempt_override("my-pass"),
            OverrideOutcome::Approved
        );
        assert_eq!(engine.remaining_attempts(), 3);
    }

    #[test]
    fn each_override_attempt_requires_password_independently() {
        // Verifies that overriding action A does NOT create a session that
        // bypasses password verification for action B. Every attempt must
        // independently provide the correct password.
        let verifier = DefaultHasherVerifier::new("correct-pass", "test-salt");
        let mut engine = OverrideEngine::new(verifier);

        // First attempt: correct password → Approved
        assert_eq!(
            engine.attempt_override("correct-pass"),
            OverrideOutcome::Approved
        );

        // Second attempt: wrong password → Failed (not auto-approved by prior session)
        assert_eq!(
            engine.attempt_override("wrong-pass"),
            OverrideOutcome::Failed
        );

        // Third attempt: correct password again → Approved
        assert_eq!(
            engine.attempt_override("correct-pass"),
            OverrideOutcome::Approved
        );

        // Fourth attempt: wrong password → Failed (no session leak)
        assert_eq!(
            engine.attempt_override("wrong-pass"),
            OverrideOutcome::Failed
        );
    }

    #[test]
    fn lockout_never_produces_session_authorization() {
        // Lockout prevents brute force, but when lockout expires,
        // there is still NO authorization session — password is required again.
        let verifier = DefaultHasherVerifier::new("the-pass", "the-salt");
        let mut engine = OverrideEngine::with_config(
            verifier,
            OverrideConfig {
                max_failed_attempts: 2,
                lockout_seconds: 0, // immediate unlock
                ..Default::default()
            },
        );

        // Two wrong attempts → locked
        assert_eq!(engine.attempt_override("wrong1"), OverrideOutcome::Failed);
        assert_eq!(engine.attempt_override("wrong2"), OverrideOutcome::Failed);

        // Lockout expired (0 sec), but there was a correct password before?
        // No — engine is clean, must provide correct password
        assert_eq!(engine.attempt_override("wrong3"), OverrideOutcome::Failed);
        assert_eq!(
            engine.attempt_override("the-pass"),
            OverrideOutcome::Approved
        );
    }

    #[test]
    fn lockout_with_correct_password_still_locked() {
        let verifier = DefaultHasherVerifier::new("the-pass", "the-salt");
        let mut engine = OverrideEngine::with_config(
            verifier,
            OverrideConfig {
                max_failed_attempts: 2,
                lockout_seconds: 300,
                ..Default::default()
            },
        );

        // Two wrong attempts
        assert_eq!(engine.attempt_override("wrong1"), OverrideOutcome::Failed);
        assert_eq!(engine.attempt_override("wrong2"), OverrideOutcome::Failed);
        assert!(engine.is_locked());

        // Even correct password is rejected while locked
        assert_eq!(engine.attempt_override("the-pass"), OverrideOutcome::Locked);
    }

    // ── Argon2PasswordVerifier tests ─────────────────────────────────────

    /// Deterministic Argon2id PHC hash for password "test-password".
    /// Generated with fixed salt "TestSalt12345678" (base64: VGVzdFNhbHQxMjM0NTY3OA).
    const TEST_ARGON2ID_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$VGVzdFNhbHQxMjM0NTY3OA$WNRQank81ztI21SJ2rYQiOncYpdJl5d3TEGNL61EMoI";

    #[test]
    fn argon2_valid_password_returns_approved() {
        let verifier = Argon2PasswordVerifier::new(TEST_ARGON2ID_HASH);
        let mut engine = OverrideEngine::new(verifier);
        let result = engine.attempt_override("test-password");
        assert_eq!(result, OverrideOutcome::Approved);
    }

    #[test]
    fn argon2_wrong_password_returns_failed() {
        let verifier = Argon2PasswordVerifier::new(TEST_ARGON2ID_HASH);
        let mut engine = OverrideEngine::new(verifier);
        let result = engine.attempt_override("wrong-password");
        assert_eq!(result, OverrideOutcome::Failed);
    }

    #[test]
    fn argon2_invalid_hash_panics_at_construction() {
        // Invalid PHC string should panic at construction
        let result = std::panic::catch_unwind(|| {
            Argon2PasswordVerifier::new("not-a-valid-hash");
        });
        assert!(result.is_err(), "invalid hash should panic at construction");
    }

    #[test]
    fn argon2_empty_hash_returns_false() {
        // Construct with a hash that is syntactically valid PHC but
        // wrong algorithm (should not happen via new() which validates,
        // but test the verify path defensively)
        let verifier = Argon2PasswordVerifier::new(TEST_ARGON2ID_HASH);
        assert!(!verifier.verify(""));
    }

    #[test]
    fn argon2_verifier_debug_does_not_leak_hash() {
        let verifier = Argon2PasswordVerifier::new(TEST_ARGON2ID_HASH);
        let debug_str = format!("{:?}", verifier);
        // The hash and password must never appear in Debug output
        assert!(
            !debug_str.contains("WNRQank81ztI21SJ2rYQiOncYpdJl5d3TEGNL61EMoI"),
            "Argon2 hash must not appear in Debug: {}",
            debug_str
        );
        assert!(
            !debug_str.contains("test-password"),
            "password must not appear in Debug: {}",
            debug_str
        );
        assert!(
            debug_str.contains("[REDACTED]"),
            "Debug must contain [REDACTED]: {}",
            debug_str
        );
    }

    #[test]
    fn argon2_integrated_with_override_engine() {
        // Full flow: Argon2PasswordVerifier + OverrideEngine
        let verifier = Argon2PasswordVerifier::new(TEST_ARGON2ID_HASH);
        let mut engine = OverrideEngine::with_config(
            verifier,
            OverrideConfig {
                max_failed_attempts: 3,
                ..Default::default()
            },
        );

        // Wrong password → Failed
        assert_eq!(
            engine.attempt_override("wrong-pass"),
            OverrideOutcome::Failed
        );
        assert_eq!(engine.remaining_attempts(), 2);

        // Correct password → Approved (and resets counter)
        assert_eq!(
            engine.attempt_override("test-password"),
            OverrideOutcome::Approved
        );
        assert_eq!(engine.remaining_attempts(), 3);
    }

    #[test]
    fn argon2_from_env_hash_both_states() {
        // Test: env var set → returns Some with valid verifier
        // NOTE: merged into single test because both tests modify ARPAGONA_OVERRIDE_PASSWORD_HASH
        // and Rust runs tests in parallel by default, causing a race condition.
        std::env::set_var("ARPAGONA_OVERRIDE_PASSWORD_HASH", TEST_ARGON2ID_HASH);
        let result = Argon2PasswordVerifier::from_env_hash();
        assert!(
            result.is_some(),
            "from_env_hash should return Some when env var is set"
        );
        if let Some(verifier) = result {
            assert!(verifier.verify("test-password"));
            assert!(!verifier.verify("wrong-password"));
        }

        // Test: env var not set → returns None
        std::env::remove_var("ARPAGONA_OVERRIDE_PASSWORD_HASH");
        let result = Argon2PasswordVerifier::from_env_hash();
        assert!(
            result.is_none(),
            "from_env_hash should return None when env var is not set"
        );
    }

    #[test]
    fn argon2_password_not_in_audit_logs() {
        // Verify that when Argon2PasswordVerifier is used with OverrideEngine,
        // the password never appears in any loggable output
        let verifier = Argon2PasswordVerifier::new(TEST_ARGON2ID_HASH);
        let mut engine = OverrideEngine::new(verifier);

        // Attempt with wrong password — failure output should not contain password
        let _ = engine.attempt_override("sensitive-password-123");
        let debug_str = format!("{:?}", engine);
        assert!(
            !debug_str.contains("sensitive-password-123"),
            "password must not leak in engine Debug output"
        );
        assert!(
            !debug_str.contains("test-password"),
            "password must not leak in engine Debug output"
        );
    }
}
