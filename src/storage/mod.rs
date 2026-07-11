mod discovery;
#[cfg(target_os = "linux")]
mod events;
mod model;

pub use discovery::discover_storage;
#[cfg(target_os = "linux")]
pub use events::spawn_block_event_hints;
#[cfg(test)]
pub(crate) use model::{Generation, Materialization, StorageNode};
pub use model::{StorageInventory, StorageKind};
