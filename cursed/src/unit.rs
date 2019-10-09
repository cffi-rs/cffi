use super::{FromForeign, InputType, ReturnType, ToForeign};

pub struct UnitMarshaler;

impl InputType for UnitMarshaler {
    type Foreign = ();
}

impl ReturnType for UnitMarshaler {
    type Foreign = ();

    #[inline(always)]
    fn foreign_default() -> () {
        ()
    }
}
