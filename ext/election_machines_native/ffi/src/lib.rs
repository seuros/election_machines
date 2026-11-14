//! Thin binding to lease-backed nomination; no storage, clocks or I/O.
mod classes;
mod election;
mod init;
mod names;
mod values;

use std::{cmp::Ordering, time::Duration};

use election_machines::{Checkpoint, FencingToken, Lease};
use magnus::{Error, IntoValue, Ruby, Value};
use matryona_machines::ruby::{integer, nanoseconds};

/// A fencing token as its raw Integer: scope above a positive sequence.
fn raw_token(value: Value) -> Result<FencingToken, Error> {
    FencingToken::from_raw(integer(value, "token")?).ok_or_else(|| {
        Error::new(
            Ruby::get_with(value).exception_range_error(),
            "token must be in 1..=2**64-1 with a positive sequence number",
        )
    })
}

/// `ElectionMachines.native_raw`: what `FencingToken.new` validates.
fn validate_raw(raw: Value) -> Result<(), Error> {
    raw_token(raw).map(drop)
}

/// A lease as `[owner, token, expires_at]`, for Ruby to build a `Lease` of.
fn row(ruby: &Ruby, lease: Lease) -> Value {
    (
        lease.owner().value(),
        lease.token().raw(),
        nanoseconds(ruby, lease.expires_at()),
    )
        .into_value_with(ruby)
}

/// `Checkpoint::from_parts`, or the `RangeError` its `None` stands for.
fn checkpoint(
    ruby: &Ruby,
    scope: u16,
    issued: u64,
    held: Option<Lease>,
    observed: Duration,
) -> Result<Checkpoint, Error> {
    Checkpoint::from_parts(scope, issued, held, observed).ok_or_else(|| {
        Error::new(
            ruby.exception_range_error(),
            "issued is beyond the scope's token space, or the lease is not the grant \
             that history ends with",
        )
    })
}

/// `FencingToken#scope`; reading the token is what validates it.
fn token_scope(ruby: &Ruby, held: Value) -> Result<u16, Error> {
    values::token(ruby, held).map(FencingToken::scope)
}

/// `FencingToken#order`: -1, 0, 1, or `nil` across scopes.
fn token_order(ruby: &Ruby, held: Value, other: Value) -> Result<Option<i8>, Error> {
    let (held, other) = (values::token(ruby, held)?, values::token(ruby, other)?);
    Ok(held.order(other).map(|order| match order {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }))
}

/// `FencingToken#newer_than?`.
fn token_newer(ruby: &Ruby, held: Value, other: Value) -> Result<bool, Error> {
    let (held, other) = (values::token(ruby, held)?, values::token(ruby, other)?);
    Ok(held.is_newer_than(other))
}
