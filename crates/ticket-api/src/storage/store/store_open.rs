use super::*;
use memory_kernel::domain_store::StoreAccessMode;

impl TicketStore {}

#[path = "store_open/path_resolution.rs"]
mod path_resolution;

#[path = "store_open/open_profiles.rs"]
mod open_profiles;

#[path = "store_open/initialization.rs"]
mod initialization;
