//! Caller-owned sector residency, exclusive replica reservations and streaming statistics.
//!
//! No thread, process, file or integration loop is owned here. Callers must confirm
//! that a failed worker has stopped before releasing its reservation for retry.
mod checkpoint;
mod estimates;
mod job;
mod moments;
mod observation;
mod reservations;
mod scheduler;
mod state;
mod weights;
use super::streams;

pub use job::{ReplicaIdentity, SerialReturn, SerialTask};
pub use moments::ReplicaMoments;
pub use state::{
    AcceptedAllocation, SerialMethod, SerialSectorSnapshot, SerialSession, SerialSettings,
    SerialSnapshot,
};

#[cfg(test)]
mod tests;
