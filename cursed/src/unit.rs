use super::{InputType, ReturnType, ToForeign};

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

impl<E> ToForeign<Result<(), E>, ()> for UnitMarshaler {
    type Error = E;

    #[inline(always)]
    fn to_foreign(local: Result<(), E>) -> Result<(), Self::Error> {
        local
    }
}
