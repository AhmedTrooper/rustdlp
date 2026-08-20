pub mod adaptive;
pub mod http;
pub mod progress;
pub mod single;
pub mod traits;

pub use adaptive::AdaptiveDownloader;
pub use http::HttpDownloader;
pub use single::SingleStreamDownloader;
pub use traits::StreamDownloader;
