//! sqs-dispatch is a library that provides a simple way to dispatch messages
//! from an SQS queue.
//!
//! * During the dispatch, it will monitor the task, adjusting visibility of the
//!   message as needed.
//! * When the task completes, it will delete the message from the queue.

mod util;

pub mod dispatch;
pub use dispatch::*;
