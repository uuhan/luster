use std::hash::{Hash, Hasher};

use failure::{bail, Error};

use gc_arena::{Collect, Gc, GcCell, MutationContext};

use crate::constant::Constant;
use crate::opcode::OpCode;
use crate::table::Table;
use crate::thread::Thread;
use crate::types::{RegisterIndex, UpValueIndex};
use crate::value::Value;

#[derive(Debug, Collect, Clone, Copy, PartialEq, Eq)]
#[collect(require_static)]
pub enum UpValueDescriptor {
    Environment,
    ParentLocal(RegisterIndex),
    Outer(UpValueIndex),
}

#[derive(Debug, Collect)]
#[collect(empty_drop)]
pub struct FunctionProto<'gc> {
    pub fixed_params: u8,
    pub has_varargs: bool,
    pub stack_size: u16,
    pub constants: Vec<Constant<'gc>>,
    pub opcodes: Vec<OpCode>,
    pub upvalues: Vec<UpValueDescriptor>,
    pub prototypes: Vec<Gc<'gc, FunctionProto<'gc>>>,
}

#[derive(Debug, Collect, Copy, Clone)]
#[collect(require_copy)]
pub enum UpValueState<'gc> {
    Open(Thread<'gc>, usize),
    Closed(Value<'gc>),
}

#[derive(Debug, Collect, Copy, Clone)]
#[collect(require_copy)]
pub struct UpValue<'gc>(pub GcCell<'gc, UpValueState<'gc>>);

#[derive(Debug, Collect)]
#[collect(empty_drop)]
pub struct ClosureState<'gc> {
    pub proto: Gc<'gc, FunctionProto<'gc>>,
    pub upvalues: Vec<UpValue<'gc>>,
}

#[derive(Debug, Copy, Clone, Collect)]
#[collect(require_copy)]
pub struct Closure<'gc>(pub Gc<'gc, ClosureState<'gc>>);

impl<'gc> PartialEq for Closure<'gc> {
    fn eq(&self, other: &Closure<'gc>) -> bool {
        &*self.0 as *const ClosureState == &*other.0 as *const ClosureState
    }
}

impl<'gc> Eq for Closure<'gc> {}

impl<'gc> Hash for Closure<'gc> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&*self.0 as *const ClosureState).hash(state)
    }
}

impl<'gc> Closure<'gc> {
    /// Create a top-level closure, prototype must not have any upvalues besides _ENV.
    pub fn new(
        mc: MutationContext<'gc, '_>,
        proto: FunctionProto<'gc>,
        environment: Option<Table<'gc>>,
    ) -> Result<Closure<'gc>, Error> {
        let proto = Gc::allocate(mc, proto);
        let mut upvalues = Vec::new();

        if !proto.upvalues.is_empty() {
            if proto.upvalues.len() > 1 || proto.upvalues[0] != UpValueDescriptor::Environment {
                bail!("cannot use prototype with upvalues other than _ENV to create top-level closure")
            } else if let Some(environment) = environment {
                upvalues.push(UpValue(GcCell::allocate(
                    mc,
                    UpValueState::Closed(Value::Table(environment)),
                )));
            } else {
                bail!("closure requires _ENV upvalue but no environment was provided")
            }
        }

        Ok(Closure(Gc::allocate(mc, ClosureState { proto, upvalues })))
    }
}
