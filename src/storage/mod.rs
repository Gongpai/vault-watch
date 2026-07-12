mod discovery;
#[cfg(target_os = "linux")]
mod events;
pub(crate) mod model;

pub use discovery::discover_storage;
#[cfg(target_os = "linux")]
pub use events::spawn_block_event_hints;
pub use model::{
    Confidence, Materialization, StorageEdgeKind, StorageInventory, StorageKind, StorageNode,
};
