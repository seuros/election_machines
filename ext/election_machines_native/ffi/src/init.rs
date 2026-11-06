//! Private initialization boundary.
use magnus::{Error, Ruby, function, method, prelude::*};
#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("ElectionMachines")?;
    module
        .define_class("Checkpoint", ruby.class_object())?
        .undef_default_alloc_func();
    let class = module.define_class("Election", ruby.class_object())?;
    class.undef_default_alloc_func();
    class.define_singleton_method("new", function!(crate::Wrapper::new, 0))?;
    class.define_method("elect", method!(crate::Wrapper::elect, 3))?;
    class.define_method("heartbeat", method!(crate::Wrapper::heartbeat, 4))?;
    class.define_method("resign", method!(crate::Wrapper::resign, 3))?;
    class.define_method("checkpoint", method!(crate::Wrapper::checkpoint, 0))?;
    class.define_method("restore!", method!(crate::Wrapper::restore, 1))?;
    class.define_method("lease", method!(crate::Wrapper::lease, 0))
}
