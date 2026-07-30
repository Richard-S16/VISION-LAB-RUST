mod labels;
mod postprocess;
mod preprocess;
mod resources;
mod session;
mod types;
mod worker;

pub use resources::DetectorResources;
pub use types::{DetectionRequestMetadata, DetectionResponse, DetectorInfo, InitializationEvent};
pub use worker::DetectorService;

pub const INPUT_WIDTH: u32 = 384;
pub const INPUT_HEIGHT: u32 = 384;
