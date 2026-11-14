//! The one answer for every call that was not honoured.
use core::fmt;

use lease_machines::Refusal as LeaseRefusal;

/// Why a call was not honoured. Every refusal leaves the election exactly as
/// it was: no grant, no token, no observed clock moves.
///
/// [`Refusal::NoCandidates`] is the election's own reason. Every other one is
/// the embedded [`LeaseAuthority`](lease_machines::LeaseAuthority)'s, carried
/// whole in [`Refusal::Lease`]: the election copies none of the lease core's
/// names, so it has no variant it can never answer. Each method documents
/// which lease refusals can arrive from it.
///
/// A nomination under a live incumbent is not a refusal: see
/// [`Leadership::Incumbent`](crate::Leadership).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The candidate list is empty: there is nobody to nominate.
    NoCandidates,
    /// The lease authority refused, for the reason it gives.
    Lease(LeaseRefusal),
}

impl From<LeaseRefusal> for Refusal {
    fn from(refusal: LeaseRefusal) -> Self {
        Self::Lease(refusal)
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NoCandidates => "the candidate list is empty",
            Self::Lease(_) => "the lease authority refused",
        })
    }
}

/// [`Refusal::Lease`] chains to the lease refusal it carries: the message
/// says the authority refused, the source says why, so an error chain prints
/// each reason once.
impl core::error::Error for Refusal {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::NoCandidates => None,
            Self::Lease(refusal) => Some(refusal),
        }
    }
}
