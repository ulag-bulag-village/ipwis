#![allow(clippy::missing_safety_doc)]

pub mod extern_data;
pub mod extrinsics;
pub mod interrupt_id;
#[cfg(target_os = "wasi")]
pub mod interrupt_id_wasi;
pub mod program;
