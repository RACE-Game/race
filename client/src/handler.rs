
#[cfg(target_arch = "wasm32")]
mod websys;
#[cfg(target_arch = "wasm32")]
pub use websys::Handler;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::Handler;
