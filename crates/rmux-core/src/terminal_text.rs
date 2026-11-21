//! Terminal grapheme measurements supplied by the pinned engine.
use serde::{Deserialize, Serialize};
/// Exclusive UTF-8 byte endpoint and terminal cell width of a whole cluster.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grapheme {
    pub end: usize,
    pub width: u8,
}
pub const MAX_MEASURE_BYTES: usize = 1024 * 1024;
