//! The slot the app keeps one index in, and the generation each scan claims a ticket from.

use ltk_manager_base::generation::{Generation, line};
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

use super::ObjectIndex;

/// The newest object search asked for, on a line of its own.
///
/// Apart from [`SearchGeneration`] so a keystroke the game scan answers never
/// gives up a scan the object rows are waiting on, and the other way round.
pub type ObjectSearchGeneration = Generation<line::ObjectSearch>;

/// The newest full search of the objects asked for, on a line of its own.
///
/// Apart from [`ObjectSearchGeneration`]. A keystroke in the objects browser's box
/// gives up no scan the palette's rows wait on.
pub type ObjectFindGeneration = Generation<line::ObjectFind>;

/// The newest reference query asked for, on a line of its own.
///
/// Apart from [`ObjectFindGeneration`]. A re-run in the References tab gives up no
/// scan the objects browser waits on.
pub type ObjectReferenceGeneration = Generation<line::References>;

/// One build's claim on the state, which a clear or a newer build revokes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildTicket(u64);

/// What an [`IndexState`] holds right now, for a search to answer from.
///
/// `E` is the shape a failed build is kept in, which the shell chooses so a
/// failure crosses IPC as it does everywhere else.
#[derive(Debug, Clone)]
pub enum IndexSnapshot<T, E> {
    /// Nothing has warmed the index, or the switch that gates it is off.
    Absent,
    /// A build is running.
    Building,
    /// The index, shared rather than locked.
    Ready(Arc<T>),
    /// The last build failed, and the next warm retries it.
    Failed(E),
}

/// What [`ObjectIndexState`] holds.
pub type ObjectIndexSnapshot<E> = IndexSnapshot<ObjectIndex, E>;

#[derive(Debug)]
enum Slot<T, E> {
    Absent,
    Building(BuildTicket),
    Ready(Arc<T>),
    Failed(E),
}

/// One app-managed index, in one of four slots.
///
/// Absent until something warms it, building while one does, and then ready
/// or failed. A build claims a ticket, and a result arriving after a clear or
/// under an older ticket is dropped, so a Rebuild or a switch-off mid-build
/// never lands an index nobody asked for.
#[derive(Debug)]
pub struct IndexState<T, E> {
    slot: Mutex<Slot<T, E>>,
    ticket: AtomicU64,
}

/// The app-managed [`ObjectIndex`].
pub type ObjectIndexState<E> = IndexState<ObjectIndex, E>;

impl<T, E> Default for IndexState<T, E> {
    fn default() -> Self {
        Self {
            slot: Mutex::new(Slot::Absent),
            ticket: AtomicU64::new(0),
        }
    }
}

impl<T, E: Clone> IndexState<T, E> {
    /// Claim the state for a build, or `None` when one is running or done.
    ///
    /// A failed slot is claimed again, so the next warm retries it.
    #[must_use]
    pub fn begin(&self) -> Option<BuildTicket> {
        let mut slot = self.slot.lock();
        if matches!(*slot, Slot::Building(_) | Slot::Ready(_)) {
            return None;
        }
        let ticket = BuildTicket(self.ticket.fetch_add(1, AtomicOrdering::Relaxed) + 1);
        *slot = Slot::Building(ticket);
        Some(ticket)
    }

    /// Whether `ticket` is still the build the state is waiting on.
    #[must_use]
    pub fn is_current(&self, ticket: BuildTicket) -> bool {
        self.ticket.load(AtomicOrdering::Relaxed) == ticket.0
    }

    /// Land a build's result, unless the state stopped waiting on it.
    pub fn finish(&self, ticket: BuildTicket, built: Result<T, E>) {
        let mut slot = self.slot.lock();
        if !matches!(*slot, Slot::Building(current) if current == ticket) {
            tracing::debug!("Dropping an index build the state stopped waiting on");
            return;
        }
        *slot = match built {
            Ok(index) => Slot::Ready(Arc::new(index)),
            Err(error) => Slot::Failed(error),
        };
    }

    /// Drop the index, and the result of any build still running.
    pub fn clear(&self) {
        let mut slot = self.slot.lock();
        self.ticket.fetch_add(1, AtomicOrdering::Relaxed);
        *slot = Slot::Absent;
    }

    /// What the state holds, with the index shared rather than locked.
    #[must_use]
    pub fn snapshot(&self) -> IndexSnapshot<T, E> {
        let slot = self.slot.lock();
        match &*slot {
            Slot::Absent => IndexSnapshot::Absent,
            Slot::Building(_) => IndexSnapshot::Building,
            Slot::Ready(index) => IndexSnapshot::Ready(Arc::clone(index)),
            Slot::Failed(error) => IndexSnapshot::Failed(error.clone()),
        }
    }
}
