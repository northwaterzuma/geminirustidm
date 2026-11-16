//! Deterministic offline fault-injection server scenarios for TEST-02A/02B/05B/06F/13A/16A/17A.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaultScenarioConfig {
    pub id: String,
    pub description: String,
    pub expected: String,
}
