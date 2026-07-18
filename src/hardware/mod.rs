pub mod interface;
pub mod razer;

// We re-export the trait so it can still be accessed cleanly via `razermatter_lib::hardware::RazerHardware`
pub use interface::RazerHardware;
