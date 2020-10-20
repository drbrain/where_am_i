mod device;
mod error;
mod fetch_future;
pub mod ioctl;

pub use device::Device;
pub use error::Error;
pub use fetch_future::FetchFuture;
pub use fetch_future::FetchTime;
