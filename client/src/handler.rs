
#[cfg(target_arch = "wasm32")]
mod handler_web;
#[cfg(target_arch = "wasm32")]
pub use handler_web::Handler;

#[cfg(not(target_arch = "wasm32"))]
mod handler_wasmer;
#[cfg(not(target_arch = "wasm32"))]
pub use handler_wasmer::Handler;
