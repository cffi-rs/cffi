use proc_macro2::TokenStream;
use quote::quote;
use std::{error::Error, fmt::Display, marker::PhantomData, sync::Arc};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    FnArg, Pat, PatType, Type,
};

pub type ErrCallback = Option<extern "C" fn(*const libc::c_char)>;

pub trait ToForeign<Local, Foreign>: Sized {
    type Error;
    fn to_foreign(_: Local) -> Result<Foreign, Self::Error>;
    fn drop_foreign(_: Foreign) {}
}

pub trait FromForeign<Foreign, Local>: Sized {
    type Error;
    fn from_foreign(_: Foreign) -> Result<Local, Self::Error>;
    fn drop_local(_: Local) {}
}

struct BoxMarshaler<T>(PhantomData<T>);

impl<T> FromForeign<*mut T, Box<T>> for BoxMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(box_ptr: *mut T) -> Result<Box<T>, Self::Error> {
        if box_ptr.is_null() {
            // TODO: error
        }

        Ok(unsafe { Box::from_raw(box_ptr) })
    }
}

impl<T> ToForeign<Box<T>, *mut T> for BoxMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(boxed: Box<T>) -> Result<*mut T, Self::Error> {
        Ok(Box::into_raw(boxed))
    }
}

struct ArcMarshaler<T>(PhantomData<T>);

impl<T> FromForeign<*const T, Arc<T>> for ArcMarshaler<T> {
    type Error = Arc<dyn Error>;

    #[inline(always)]
    fn from_foreign(arc_ptr: *const T) -> Result<Arc<T>, Self::Error> {
        if arc_ptr.is_null() {
            // TODO: error
        }

        Ok(unsafe { Arc::from_raw(arc_ptr) })
    }
}

impl<T> ToForeign<Arc<T>, *const T> for ArcMarshaler<T> {
    type Error = Arc<dyn Error>;

    #[inline(always)]
    fn to_foreign(arced: Arc<T>) -> Result<*const T, Self::Error> {
        Ok(Arc::into_raw(arced))
    }
}

pub struct BoolMarshaler;

impl FromForeign<u8, bool> for BoolMarshaler {
    type Error = std::convert::Infallible;

    #[inline(always)]
    fn from_foreign(i: u8) -> Result<bool, Self::Error> {
        Ok(i != 0)
    }
}

use std::{
    borrow::Cow,
    ffi::{CStr, CString},
};

pub struct StrMarshaler<'a>(&'a PhantomData<()>);

impl<'a> FromForeign<*const libc::c_char, Cow<'a, str>> for StrMarshaler<'a> {
    type Error = Box<dyn Error>;

    fn from_foreign(key: *const libc::c_char) -> Result<Cow<'a, str>, Self::Error> {
        Ok(unsafe { CStr::from_ptr(key) }.to_string_lossy())
    }
}

impl<'a> ToForeign<&'a str, *const libc::c_char> for StrMarshaler<'a> {
    type Error = Box<dyn Error>;

    fn to_foreign(input: &'a str) -> Result<*const libc::c_char, Self::Error> {
        let c_str = CString::new(input)?;
        Ok(c_str.into_raw())
    }

    fn drop_foreign(ptr: *const libc::c_char) {
        unsafe { CString::from_raw(ptr as *mut _) };
    }
}

impl ToForeign<bool, u8> for BoolMarshaler {
    type Error = std::convert::Infallible;

    #[inline(always)]
    fn to_foreign(b: bool) -> Result<u8, Self::Error> {
        Ok(if b { 1 } else { 0 })
    }
}

fn collect_mappings_from_sig(
    sig: &mut syn::Signature,
) -> Result<Vec<(PatType, Option<syn::Path>)>, syn::Error> {
    if let Some(syn::FnArg::Receiver(item)) = sig.inputs.first() {
        return Err(syn::Error::new(item.span(), "Cannot support self"));
    }

    let attrs = sig
        .inputs
        .iter_mut()
        .filter_map(|x| match x {
            syn::FnArg::Typed(t) => Some(t),
            _ => None,
        })
        .map(|input| {
            let mut unhandled_attrs = vec![];

            std::mem::swap(&mut input.attrs, &mut unhandled_attrs);

            let mut idents = unhandled_attrs
                .into_iter()
                .filter_map(|item| {
                    // Try to get the item as a Meta
                    let meta = match item.parse_meta() {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("Meta yolo: {:?}: {:?}", &item, e);
                            input.attrs.push(item);
                            return None;
                        }
                    };

                    let list = match meta {
                        syn::Meta::List(list) => list,
                        _ => {
                            input.attrs.push(item);
                            return None;
                        }
                    };

                    if list.nested.len() > 1 {
                        // TODO: throw proper error
                        input.attrs.push(item);
                        return None;
                    }

                    let marshaler = match list.nested.first() {
                        Some(syn::NestedMeta::Meta(syn::Meta::Path(v))) => v,
                        _ => {
                            // TODO: throw proper error
                            input.attrs.push(item);
                            return None;
                        }
                    };

                    Some(marshaler.to_owned())
                })
                .collect::<Vec<_>>();

            if idents.len() > 1 {
                // TODO: have a very strong, negative opinion.
            }

            (input.to_owned(), idents.pop())
        })
        .map(|x| {
            let default_marshaler = DEFAULT_MARSHALERS.with(|map| map.get(&x.0.ty).map(|x| x.clone()));
            (x.0, x.1.or_else(|| default_marshaler))
        })
        .collect::<Vec<_>>();

    Ok(attrs)
}

fn process_function(
    mut func: syn::ItemFn,
) -> Result<(syn::ItemFn, Vec<(PatType, Option<syn::Path>)>), syn::Error> {
    // Dig into inputs
    let marshal_attrs = collect_mappings_from_sig(&mut func.sig)?;

    Ok((func, marshal_attrs))
}

use darling::FromMeta;

#[derive(Debug, FromMeta, Default)]
pub struct InvokeParams {
    pub return_marshaler: Option<syn::Path>
}

pub fn call_with(
    invoke_params: InvokeParams,
    raw_function: TokenStream,
) -> Result<TokenStream, syn::Error> {
    let function: syn::Item =
        syn::parse2(raw_function.clone()).context("error parsing function body")?;
    let (fn_item, marshalers) = match function {
        syn::Item::Fn(f) => process_function(f)?,
        _ => {
            return Err(syn::Error::new_spanned(
                &raw_function,
                "only supported on functions right now",
            ))
        }
    };

    let syn::Signature { ident: ref name, inputs: ref params, output: ref return_type, .. } =
        fn_item.sig.clone();

    let c_return_type = match return_type {
        syn::ReturnType::Type(x, ty) => syn::ReturnType::Type(x.clone(), Box::new(to_c_type(&*ty)?)),
        x => x.clone(),
    };

    let return_type_ty = match &return_type {
        syn::ReturnType::Type(_, ty) => Some(*ty.clone()),
        _ => None
    };

    let c_return_type_ty = match &c_return_type {
        syn::ReturnType::Type(_, ty) => Some(*ty.clone()),
        _ => None
    };

    let mut from_foreigns = TokenStream::new();
    let mut c_params: Punctuated<PatType, syn::Token![,]> = Punctuated::new();
    let mut c_args: Punctuated<Pat, syn::Token![,]> = Punctuated::new();

    for (i, param) in params.iter().enumerate() {
        let (_out_type, marshaler) = &marshalers[i];
        let name = to_c_arg(param).context("failed to convert Rust type to FFI type")?;
        let in_type = to_c_param(param).context("failed to convert Rust type to FFI type")?;

        if let Some(marshaler) = marshaler {
            let foreign = gen_foreign(&marshaler, &name, c_return_type_ty.as_ref());
            from_foreigns.extend(foreign);
        } else {
            return Err(syn::Error::new_spanned(
                &param,
                "no marshaler found for type",
            ))
        }

        c_params.push(in_type);
        c_args.push(name);
    }

    c_params.push(PatType {
        attrs: vec![],
        pat: Box::new(Pat::Verbatim(quote! { __exception })),
        colon_token: <syn::Token![:]>::default(),
        ty: Box::new(Type::Verbatim(quote! { ::cthulhu::ErrCallback })),
    });

    let rust_fn = &raw_function;
    println!("RET: {:?}", &return_type);
    
    match return_type {
        syn::ReturnType::Default => {
            Ok(quote! {
                #[no_mangle]
                extern "C" fn #name(#c_params) {
                    #from_foreigns
                    #rust_fn
                    #name(#c_args);
                }
            })
        },
        syn::ReturnType::Type(_, _) => {
            let return_marshaler = invoke_params.return_marshaler.map(|x| Ok(x)).unwrap_or_else(|| {
                DEFAULT_MARSHALERS.with(|map| {
                    map.get(&return_type_ty.unwrap()).map(|x| x.clone()).ok_or_else(|| {
                        syn::Error::new_spanned(
                            &return_type,
                            "no marshaler found for type",
                        )
                    })
                })
            })?;

            let return_to_foreign = quote! {
                match #return_marshaler::to_foreign(result) {
                    Ok(v) => v,
                    Err(e) => ::cthulhu::throw!(e, u8)
                }
            };

            Ok(quote! {
                #[no_mangle]
                extern "C" fn #name(#c_params) #c_return_type {
                    #from_foreigns
                    #rust_fn
                    let result = #name(#c_args);
                    #return_to_foreign
                }
            })
        }
    }



}

fn to_c_param(arg: &FnArg) -> Result<PatType, syn::Error> {
    match arg {
        FnArg::Typed(arg) => Ok(PatType { ty: Box::new(to_c_type(&*arg.ty)?), ..arg.clone() }.into()),
        _ => Err(syn::Error::new(arg.span(), "cannot")),
    }
}

fn to_c_arg(arg: &FnArg) -> Result<Pat, syn::Error> {
    match arg {
        FnArg::Typed(arg) => Ok(*arg.pat.clone()),
        _ => Err(syn::Error::new(arg.span(), "cannot")),
    }
}

fn gen_foreign(
    marshaler: &syn::Path,
    name: &syn::Pat,
    ty: Option<&syn::Type>
) -> TokenStream {
    if let Some(ty) = ty {
        quote! {
            let #name = ::cthulhu::try!(
                #marshaler::from_foreign(#name),
                __exception,
                #ty
            );
        }
    } else {
        quote! {
            let #name = ::cthulhu::try!(
                #marshaler::from_foreign(#name),
                __exception
            );
        }
    }
}

fn to_c_type(ty: &Type) -> Result<Type, syn::Error> {
    TYPE_MAPPING.with(|map| {
        // let PatType { ty, pat, .. } = arg.clone();
        match &*ty {
            syn::Type::Path(..) | syn::Type::Reference(..) => {}
            _ => return Err(syn::Error::new(ty.span(), "Unknown parameters not supported")),
        }

        match map.get(&ty).cloned() {
            Some(types) => match types.as_slice() {
                [c_ty] => Ok(c_ty.clone()),
                _ => unreachable!(),
            },
            None => {
                let c_ty: Type = syn::parse2(quote! { *const ::libc::c_void }).unwrap();
                Ok(c_ty.clone())
            }
        }
    })
}

macro_rules! map_types {
    [$($rust:ty => [$($c:ty),*],)*] => {{
        let mut map = std::collections::HashMap::<Type, Vec<Type>>::new();
        $(map.insert(
            syn::parse2(quote!{ $rust })
                .expect(concat!("cannot parse", stringify!($rust), "as type")),
            vec![$(
                syn::parse2(quote!{ $c })
                    .expect(concat!("cannot parse", stringify!($c), "as type")),
            )*],
        );)*
        map
    }}
}

macro_rules! map_marshalers {
    [$($rust:ty => $c:ty,)*] => {{
        let mut map = std::collections::HashMap::<Type, syn::Path>::new();
        $(map.insert(
            syn::parse2(quote!{ $rust })
                .expect(concat!("cannot parse", stringify!($rust), "as type")),

            syn::parse2(quote!{ $c })
                .expect(concat!("cannot parse", stringify!($c), "as path")),

        );)*
        map
    }}
}

thread_local! {
    pub static DEFAULT_MARSHALERS: std::collections::HashMap<Type, syn::Path> = map_marshalers![
        bool => ::cthulhu::BoolMarshaler,
        Cow<str> => ::cthulhu::StrMarshaler,
        Arc<T> => ::cthulhu::ArcMarshaler<T>,
        Box<T> => ::cthulhu::BoxMarshaler<T>,
    ];

    pub static TYPE_MAPPING: std::collections::HashMap<Type, Vec<Type>> = map_types![
        bool => [u8],
        u8 => [::libc::c_uchar],
        i8 => [::libc::c_char],
        i16 => [::libc::c_short],
        u16 => [::libc::c_ushort],
        i32 => [::libc::c_int],
        u32 => [::libc::c_uint],
        i64 => [::libc::c_long],
        u64 => [::libc::c_ulong],
        &'a str => [*const ::libc::c_char],
        &'a CStr => [*const ::libc::c_char],
        CString => [*mut ::libc::c_char],
        Arc<str> => [*const ::libc::c_char],
        Cow<str> => [*const ::libc::c_char],
    ];
}

trait ErrorExt<T> {
    fn context(self, msg: impl Display) -> Result<T, syn::Error>;
}

impl<T> ErrorExt<T> for Result<T, syn::Error> {
    fn context(self, msg: impl Display) -> Self {
        match self {
            Err(err) => Err(syn::Error::new(err.span(), format!("{}: {}", msg, err.to_string()))),
            x => x,
        }
    }
}

#[no_mangle]
pub extern fn derp_callback(callback: Option<extern "C" fn(*const libc::c_char)>) {
    let cstr = CString::new("This is a string!").unwrap();
    
    if let Some(callback) = callback {
        callback(cstr.as_ptr());
    }
}