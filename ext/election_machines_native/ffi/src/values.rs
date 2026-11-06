//! Strict Ruby boundary validation.
use lease_machines::FencingToken;
use magnus::{Error, Float, Integer, Ruby, TryConvert, Value};
use std::time::Duration;
#[derive(Clone, Copy)]
pub struct Id(pub u64);
impl TryConvert for Id {
    fn try_convert(value: Value) -> Result<Self, Error> {
        if Integer::from_value(value).is_none() {
            return Err(Error::new(
                Ruby::get_with(value).exception_type_error(),
                "expected an Integer",
            ));
        }
        u64::try_convert(value).map(Self)
    }
}
#[derive(Clone, Copy)]
pub struct Seconds(pub Duration);
impl TryConvert for Seconds {
    fn try_convert(value: Value) -> Result<Self, Error> {
        let ruby = Ruby::get_with(value);
        if Integer::from_value(value).is_none() && Float::from_value(value).is_none() {
            return Err(Error::new(
                ruby.exception_type_error(),
                "seconds must be Integer or Float",
            ));
        }
        Duration::try_from_secs_f64(f64::try_convert(value)?)
            .map(Self)
            .map_err(|_| {
                Error::new(
                    ruby.exception_arg_error(),
                    "seconds must be finite, nonnegative and representable",
                )
            })
    }
}
pub fn token(ruby: &Ruby, value: Id) -> Result<FencingToken, Error> {
    FencingToken::new(value.0)
        .ok_or_else(|| Error::new(ruby.exception_arg_error(), "token must be positive"))
}
pub fn interval(ruby: &Ruby, now: Seconds, ttl: Seconds) -> Result<(), Error> {
    if ttl.0.is_zero() || now.0.checked_add(ttl.0).is_none() {
        return Err(Error::new(
            ruby.exception_arg_error(),
            "TTL must be positive and expiration representable",
        ));
    }
    Ok(())
}
