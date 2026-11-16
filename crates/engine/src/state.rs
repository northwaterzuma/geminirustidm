use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferMode {
    MultiSegmentRange,
    SingleStreamWithLength,
    SingleStreamUnknownLength,
}
