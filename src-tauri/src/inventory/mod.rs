pub(crate) mod commands;
mod model;
mod repository;
mod service;

pub(crate) use model::*;
pub(crate) use repository::{current_balance, insert_movement, stock_state, NewMovement};
pub(crate) use service::{InventoryError, InventoryService};
