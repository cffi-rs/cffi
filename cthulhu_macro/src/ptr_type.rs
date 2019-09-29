pub enum PtrType {
    Mut,
    Const,
}

impl PtrType {
    pub fn from(ty: Option<&syn::Type>) -> Option<PtrType> {
        match ty {
            Some(syn::Type::Ptr(ptr)) => {
                if ptr.const_token.is_some() {
                    Some(PtrType::Const)
                } else {
                    Some(PtrType::Mut)
                }
            }
            _ => None,
        }
    }
}
