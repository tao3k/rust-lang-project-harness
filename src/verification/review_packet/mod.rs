//! Review-packet protocol surface for reviewer-first evidence summaries.

mod build;
mod model;
mod render;

pub use build::build_rust_review_packet;
pub use model::{
    RUST_REVIEW_PACKET_PROTOCOL_ID, RUST_REVIEW_PACKET_PROTOCOL_VERSION,
    RUST_REVIEW_PACKET_SCHEMA_ID, RUST_REVIEW_PACKET_SCHEMA_VERSION, RustReviewPacket,
    RustReviewPacketInput, RustReviewPacketReceiptKind, RustReviewPacketWaiver,
    RustReviewPacketWaiverStatus,
};
pub use render::{render_rust_review_packet, render_rust_review_packet_json};
