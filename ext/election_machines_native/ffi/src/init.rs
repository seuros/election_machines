//! Private native entry point.
use magnus::{Error, Ruby, function, method, prelude::*};

use crate::election::Election;
use crate::{classes, names, values};

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("ElectionMachines")?;
    classes::resolve(ruby, module)?;
    let symbols = |names: &[&str]| {
        let list = ruby.ary_from_iter(names.iter().map(|name| ruby.to_symbol(*name)));
        list.freeze();
        list
    };
    module.const_set("REFUSALS", symbols(&names::REFUSALS))?;
    module.const_set("CLOCK_POLICIES", symbols(&names::CLOCK_POLICIES))?;
    module.define_singleton_method("native_token_scope", function!(crate::token_scope, 1))?;
    module.define_singleton_method("native_token_order", function!(crate::token_order, 2))?;
    module.define_singleton_method("native_token_newer", function!(crate::token_newer, 2))?;
    module.define_singleton_method("native_raw", function!(crate::validate_raw, 1))?;
    module.define_singleton_method("native_lease", function!(values::validate_lease, 3))?;
    module.define_singleton_method("native_held", function!(values::validate_held, 1))?;
    let checkpoint = function!(values::validate_checkpoint, 4);
    module.define_singleton_method("native_checkpoint", checkpoint)?;

    let class = module.define_class("Election", ruby.class_object())?;
    class.undef_default_alloc_func();
    class.define_singleton_method("native_with_scope", function!(Election::with_scope, 1))?;
    class.define_singleton_method("native_restored", function!(Election::restored, 3))?;
    class.define_method("scope", method!(Election::scope, 0))?;
    class.define_method("issued", method!(Election::issued, 0))?;
    class.define_method("clock_policy", method!(Election::clock_policy, 0))?;
    class.define_private_method(
        "native_with_clock_policy",
        method!(Election::with_clock_policy, 1),
    )?;
    class.define_private_method("native_lease", method!(Election::lease, 0))?;
    class.define_private_method("native_checkpoint", method!(Election::checkpoint, 0))?;
    class.define_private_method("native_elect", method!(Election::elect, 3))?;
    class.define_private_method("native_heartbeat", method!(Election::heartbeat, 4))?;
    class.define_private_method("native_resign", method!(Election::resign, 3))?;
    Ok(())
}
