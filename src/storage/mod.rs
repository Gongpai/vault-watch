mod discovery;
mod model;

pub use discovery::discover_storage;
#[cfg(test)]
pub(crate) use model::{Generation, Materialization, StorageNode};
pub use model::{StorageInventory, StorageKind};
