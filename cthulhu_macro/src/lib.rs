use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::quote;
use std::fmt::Display;
use syn::{punctuated::Punctuated, spanned::Spanned};
use heck::SnakeCase;

#[derive(Debug)]
struct Mapping {
    output_type: syn::Type,
    marshaler: Option<MarshalAttr>,
}

impl Mapping {
    fn self_type(parent: &syn::Type) -> Mapping {
        Mapping { output_type: parent.clone(), marshaler: MarshalAttr::self_type() }
    }
}

#[derive(Debug)]
struct MarshalAttr {
    path: syn::Path,
    types: Vec<syn::Type>
}

impl MarshalAttr {
    fn self_type() -> Option<MarshalAttr> {
        Some(MarshalAttr {
            path: syn::parse2(quote! { ::cursed::BoxMarshaler }).unwrap(),
            types: vec![]
        })
    }

    fn first_type(&self) -> Option<syn::Type> {
        self.types.first().cloned()
    }

    fn return_type(&self) -> Option<syn::ReturnType> {
        self.first_type().map(|ty| 
            syn::ReturnType::Type(<syn::Token![->]>::default(), Box::new(ty))
        )
    }

    fn from_defaults_by_type(ty: &syn::Type) -> Option<MarshalAttr> {
        default_marshaler(&ty).map(|x| MarshalAttr { path: x.clone(), types: vec![] })
    }

    fn from_defaults_by_return_type(ty: &syn::ReturnType) -> Option<MarshalAttr> {
        match ty {
            syn::ReturnType::Type(_, ty) => Self::from_defaults_by_type(&*ty),
            _ => None,
        }
    }

    fn from_path(path: syn::Path) -> Result<Option<MarshalAttr>, syn::Error> {
        let types = path.segments.iter().filter_map(|s| match &s.arguments {
            syn::PathArguments::AngleBracketed(args) => {
                match args.args.first() {
                    Some(syn::GenericArgument::Type(ty)) => Some(ty.clone()),
                    _ => None
                }
            },
            _ => None
        }).collect::<Vec<_>>();

        Ok(Some(MarshalAttr {
            path,
            types
        }))
    }

    fn from_attribute(attr: syn::Attribute) -> Result<Option<MarshalAttr>, syn::Error> {
        match attr.path.segments.first() {
            Some(v) => {
                if v.ident.to_string() != "marshal" {
                    return Ok(None);
                }
            },
            None => {
                return Ok(None);
            }
        }

        let marshal_ty: syn::Type = match syn::parse2(attr.tokens) {
            Ok(v) => v,
            Err(e) => {
                return Err(e)
            }
        };

        let path: syn::Path = match marshal_ty {
            syn::Type::Paren(path) => match *path.elem {
                syn::Type::Path(path) => path.path,
                e => {
                    return Err(syn::Error::new(e.span(), "Must be a path"));
                }
            }
            e => {
                return Err(syn::Error::new(e.span(), "Must be a path"));
            }
        };

        Self::from_path(path)
    }
}

fn collect_mappings_from_sig(
    parent_type: Option<&syn::Type>,
    sig: &mut syn::Signature,
) -> Result<Vec<Mapping>, syn::Error> {
    let attrs = sig
        .inputs
        .iter_mut()
        .filter_map(|mut input| {
            let input = match &mut input {
                syn::FnArg::Typed(t) => t,
                syn::FnArg::Receiver(receiver) => {
                    if let Some(parent_type) = parent_type {
                        return Some(Ok(Mapping::self_type(parent_type)))
                    } else {
                        return Some(Err(syn::Error::new_spanned(&receiver, "no self type found; using invoke wrong?")));
                    }
                },
            };

            println!("{}", quote!{ #input });
            let mut unhandled_attrs = vec![];

            std::mem::swap(&mut input.attrs, &mut unhandled_attrs);

            let idents = unhandled_attrs
                .into_iter()
                .filter_map(|item| {
                    match MarshalAttr::from_attribute(item.clone()) {
                        Ok(None) => {
                            input.attrs.push(item);
                            return None;
                        },
                        Ok(Some(v)) => Some(Ok(v)),
                        Err(e) => return Some(Err(e))
                    }
                })
                .collect::<Result<Vec<_>, _>>();

            let mut idents = match idents {
                Ok(v) => v,
                Err(e) => return Some(Err(e))
            };

            let marshaler = idents.pop()
                .or_else(|| MarshalAttr::from_defaults_by_type(&input.ty));

            Some(Ok(Mapping {
                output_type: *input.ty.clone(),
                marshaler,
            }))
        })
        .collect::<Result<Vec<_>, _>>();

    attrs
}

#[derive(Debug, FromMeta, Default)]
pub struct InvokeParams {
    #[darling(default)]
    pub return_marshaler: Option<syn::Path>,
    #[darling(default)]
    pub prefix: Option<String>,
}

enum PtrType {
    Mut,
    Const,
}

impl PtrType {
    fn from(ty: Option<&syn::Type>) -> Option<PtrType> {
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

pub fn call_with(
    invoke_params: InvokeParams,
    item: TokenStream,
) -> Result<TokenStream, syn::Error> {
    let item: syn::Item = syn::parse2(item.clone()).context("error parsing function body")?;
    match item {
        syn::Item::Fn(item) => call_with_function(invoke_params.return_marshaler, item, None),
        syn::Item::Impl(item) => call_with_impl(invoke_params.prefix, item),
        _ => Err(syn::Error::new_spanned(&item, "Only supported on functions and impls")),
    }
}

fn call_with_impl(
    prefix: Option<String>,
    item: syn::ItemImpl,
) -> Result<TokenStream, syn::Error> {
    if let Some(defaultness) = item.defaultness {
        return Err(syn::Error::new_spanned(&defaultness, "Does not support specialised impls"));
    }

    if let Some(unsafety) = item.unsafety {
        return Err(syn::Error::new_spanned(&unsafety, "Does not support unsafe impls"));
    }

    if !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(&item.generics, "Does not support generic impls"));
    }

    if let Some(trait_) = item.trait_ {
        return Err(syn::Error::new_spanned(&trait_.1, "Does not support trait impls"));
    }

    let self_ty = item.self_ty;
    let invoke_prefix = prefix.unwrap_or_else(|| "".into());
    let prefix = format!("{}_{}", invoke_prefix, quote! { #self_ty }).to_snake_case();
    let pub_methods = item.items.into_iter().filter_map(|impl_item| match impl_item {
        syn::ImplItem::Method(method) => match (
            &method.vis,
            method.sig.asyncness,
            method.sig.unsafety,
            &method.sig.abi,
            &method.sig.generics.params.is_empty(),
        ) {
            (syn::Visibility::Public(_), None, None, None, true) => Some(method),
            _ => None,
        },
        _ => None,
    });

    let pub_method_names = pub_methods
        .map(|mut x| {
            let ident = &x.sig.ident;
            let fn_path: syn::Path = syn::parse2(quote! { #self_ty::#ident })?;
            let c_ident: syn::Ident = syn::parse_str(&format!("{}_{}", prefix, &ident).to_snake_case()).unwrap();
            // let mut fn_item = syn::ItemFn {
            //     attrs: vec![],
            //     vis: syn::Visibility::Inherited,
            //     sig: x.sig,
            //     block: Box::new(x.block)
            // };
            let mappings = collect_mappings_from_sig(Some(&*self_ty), &mut x.sig)?;
            
            let mut idents = x.attrs
                .into_iter()
                .filter_map(|item| {
                    match MarshalAttr::from_attribute(item.clone()) {
                        Ok(None) => {
                            // fn_item.attrs.push(item);
                            return None;
                        },
                        Ok(Some(v)) => Some(Ok(v)),
                        Err(e) => return Some(Err(e))
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            let attr = idents.pop();

            let syn::Signature {
                ident: name,
                inputs: params,
                output: local_return_type,
                ..
            } = x.sig.clone();

            let fn_marshal_attr = match attr.map(|x| x.path) {
                Some(p) => MarshalAttr::from_path(p)?,
                None => MarshalAttr::from_defaults_by_return_type(&local_return_type)
            };

            let return_type = ReturnType::new(fn_marshal_attr.as_ref(), local_return_type)?;
            let function = Function::new(c_ident, params, &mappings, return_type, InnerFn::FunctionCall(fn_path), fn_marshal_attr)?;

            function.to_token_stream()
        })
        .collect::<Result<Vec<_>, _>>()?;

    let free_ident: syn::Ident = syn::parse_str(&format!("{}_free", prefix).to_snake_case()).unwrap();

    Ok(quote! {
        #[no_mangle]
        pub extern "C" fn #free_ident(
            __handle: *mut ::libc::c_void,
            __exception: ::cursed::ErrCallback,
        ) {
            let _: Box<#self_ty> = ::cursed::try_not_null!(
                ::cursed::BoxMarshaler::from_foreign(__handle),
                __exception
            );
            log::debug!("`{}` has consumed this handle; do not reuse it!", stringify!(#free_ident));
            unsafe {
                *__handle = std::ptr::null_mut();
            }
        }

        #(#pub_method_names)*
    })
}

struct ReturnType {
    local: syn::ReturnType,
    foreign: syn::ReturnType,
}

impl ReturnType {
    fn new(marshal_attr: Option<&MarshalAttr>, local: syn::ReturnType) -> Result<ReturnType, syn::Error> {
        let foreign = match (
            marshal_attr.and_then(|x| x.return_type()),
            &local
        ) {
            (Some(ty), _) => ty,
            (_, syn::ReturnType::Type(x, ty)) => {
                syn::ReturnType::Type(x.clone(), Box::new(to_c_type(&*ty)?))
            }
            (_, x) => x.clone(),
        };

        Ok(ReturnType { foreign, local })
    }

    fn local_type(&self) -> Option<syn::Type> {
        match &self.local {
            syn::ReturnType::Type(_, ty) => Some(*ty.clone()),
            _ => None,
        }
    }

    fn foreign_type(&self) -> Option<syn::Type> {
        match &self.foreign {
            syn::ReturnType::Type(_, ty) => Some(*ty.clone()),
            _ => None,
        }
    }

    fn foreign_ptr_type(&self) -> Option<PtrType> {
        PtrType::from(self.foreign_type().as_ref())
    }
}

enum InnerFn {
    FunctionBody(syn::ItemFn),
    FunctionCall(syn::Path)
}

struct Function {
    name: syn::Ident,
    foreign_params: Punctuated<syn::PatType, syn::Token![,]>,
    foreign_args: Punctuated<syn::Pat, syn::Token![,]>,
    return_type: ReturnType,
    from_foreigns: TokenStream,
    inner_fn: InnerFn,
    fn_marshal_attr: Option<MarshalAttr>,
}

impl Function {
    fn new(
        name: syn::Ident,
        params: Punctuated<syn::FnArg, syn::Token![,]>,
        mappings: &[Mapping],
        return_type: ReturnType,
        inner_fn: InnerFn,
        fn_marshal_attr: Option<MarshalAttr>,
    ) -> Result<Function, syn::Error> {
        let mut from_foreigns = TokenStream::new();
        let mut foreign_params: Punctuated<syn::PatType, syn::Token![,]> = Punctuated::new();
        let mut foreign_args: Punctuated<syn::Pat, syn::Token![,]> = Punctuated::new();

        let mut has_exceptions = false;

        for (i, param) in params.iter().enumerate() {
            let mapping = &mappings[i];
            let out_type = &mapping.output_type;
            let marshaler = &mapping.marshaler;

            let name = to_c_arg(param).context("failed to convert Rust type to FFI type")?;
            let mut in_type = to_c_param(param).context("failed to convert Rust type to FFI type")?;

            if let Some(in_ty_override) = marshaler.as_ref().and_then(|m| m.types.first().cloned()) {
                in_type.ty = Box::new(in_ty_override);
            }

            if let Some(marshaler) = mapping.marshaler.as_ref() {
                let foreign = gen_foreign(&marshaler.path, &name, &out_type, return_type.foreign_type().as_ref());
                from_foreigns.extend(foreign);
                has_exceptions = true;
            } else if !is_passthrough_type(&*out_type) {
                println!("OUT TY: {}, {:?}", quote! { #name }, &*out_type);
                println!("{:?}", mapping);
                println!("{:?}", fn_marshal_attr);
                return Err(syn::Error::new_spanned(&param, "no marshaler found for type"));
            }

            foreign_params.push(in_type);
            foreign_args.push(name);
        }

        let passthrough_return = return_type.local_type()
            .map(|ty| is_passthrough_type(&ty))
            .unwrap_or(true);

        if has_exceptions || !passthrough_return {
            foreign_params.push(syn::PatType {
                attrs: vec![],
                pat: Box::new(syn::Pat::Verbatim(quote! { __exception })),
                colon_token: <syn::Token![:]>::default(),
                ty: Box::new(syn::Type::Verbatim(quote! { ::cursed::ErrCallback })),
            });
        }

        Ok(Function {
            name,
            foreign_params,
            foreign_args,
            return_type,
            from_foreigns,
            inner_fn,
            fn_marshal_attr,
        })
    }

    fn to_token_stream(&self) -> Result<TokenStream, syn::Error> {
        let Self {
            name,
            foreign_params,
            foreign_args,
            return_type,
            from_foreigns,
            inner_fn,
            fn_marshal_attr,
        } = self;
        
        let call_name: syn::Path = match inner_fn {
            InnerFn::FunctionCall(path) => path.clone(),
            _ => syn::parse2(quote! { #name }).unwrap()
        };

        let original_fn = match inner_fn {
            InnerFn::FunctionBody(body) => Some(body),
            _ => None
        };

        match &return_type.local {
            syn::ReturnType::Default => Ok(quote! {
                #[no_mangle]
                pub extern "C" fn #name(#foreign_params) {
                    #from_foreigns
                    #original_fn
                    #call_name(#foreign_args);
                }
            }),
            syn::ReturnType::Type(_, ty) => {
                let foreign_return = &return_type.foreign;
                
                if is_passthrough_type(&ty) {
                    return Ok(quote! {
                        #[no_mangle]
                        extern "C" fn #name(#foreign_params) #foreign_return {
                            #from_foreigns
                            #original_fn
                            #call_name(#foreign_args)
                        }
                    });
                }

                let return_marshaler = match fn_marshal_attr.as_ref() {
                    Some(v) => &v.path,
                    None => {
                        return Err(syn::Error::new_spanned(
                            &ty,
                            "no marshaler found for return type",
                        ))
                    }
                };

                let err = match return_type.foreign_ptr_type() {
                    None => {
                        match return_type.foreign_type() {
                            Some(ret) => quote! { ::cursed::throw!(e, __exception, <#ret>::default()) },
                            None => quote! { ::cursed::throw!(e, __exception) }
                        }
                    },
                    Some(PtrType::Const) => {
                        quote! { ::cursed::throw!(e, __exception, std::ptr::null()) }
                    }
                    Some(PtrType::Mut) => {
                        quote! { ::cursed::throw!(e, __exception, std::ptr::null_mut()) }
                    }
                };

                let return_to_foreign = quote! {
                    match #return_marshaler::to_foreign(result) {
                        Ok(v) => v,
                        Err(e) => #err
                    }
                };

                Ok(quote! {
                    #[no_mangle]
                    pub extern "C" fn #name(#foreign_params) #foreign_return {
                        #from_foreigns
                        #original_fn
                        let result = #call_name(#foreign_args);
                        #return_to_foreign
                    }
                })
            }
        }
    }
}

fn call_with_function(
    return_marshaler: Option<syn::Path>,
    mut fn_item: syn::ItemFn,
    parent_type: Option<&syn::Type>
) -> Result<TokenStream, syn::Error> {
    let mappings = collect_mappings_from_sig(parent_type, &mut fn_item.sig)?;

    let syn::Signature {
        ident: name,
        inputs: params,
        output: local_return_type,
        ..
    } = fn_item.sig.clone();

    let fn_marshal_attr = match return_marshaler {
        Some(p) => MarshalAttr::from_path(p)?,
        None => MarshalAttr::from_defaults_by_return_type(&local_return_type)
    };

    let return_type = ReturnType::new(fn_marshal_attr.as_ref(), local_return_type)?;
    let function = Function::new(name, params, &mappings, return_type, InnerFn::FunctionBody(fn_item), fn_marshal_attr)?;

    function.to_token_stream()
}

fn to_c_param(arg: &syn::FnArg) -> Result<syn::PatType, syn::Error> {
    match arg {
        syn::FnArg::Typed(arg) => {
            Ok(syn::PatType { ty: Box::new(to_c_type(&*arg.ty)?), ..arg.clone() }.into())
        }
        syn::FnArg::Receiver(receiver) => {
            Ok(syn::PatType {
                attrs: vec![],
                pat: syn::parse2(quote! { __handle }).unwrap(),
                colon_token: <syn::Token![:]>::default(),
                ty: Box::new(syn::parse2(quote! { *const ::libc::c_void }).unwrap())
            })
        }
    }
}

fn to_c_arg(arg: &syn::FnArg) -> Result<syn::Pat, syn::Error> {
    match arg {
        syn::FnArg::Typed(arg) => Ok(*arg.pat.clone()),
        syn::FnArg::Receiver(receiver) => {
            Ok(syn::parse2(quote! { __handle }).unwrap())
        }
    // if let Some(syn::FnArg::Receiver(_)) = sig.inputs.first() {
    //     // TODO: handle mutability, etc on self
    //     self_ref = Some(syn::PatType {
    //         attrs: vec![],
    //         pat: syn::parse2(quote! { __handle }).unwrap(),
    //         colon_token: <syn::Token![:]>::default(),
    //         ty: Box::new(syn::parse2(quote! { *const ::libc::c_void }).unwrap())
    //     });
    // }
        // e => {
        //     println!("{:?}", e);
        //     Err(syn::Error::new(arg.span(), "cannot arg"))
        // }
    }
}

fn gen_foreign(marshaler: &syn::Path, name: &syn::Pat, out_ty: &syn::Type, ret_ty: Option<&syn::Type>) -> TokenStream {
    if let Some(ty) = ret_ty {
        let fallback = match PtrType::from(Some(ty)) {
            None => quote! { <#ty>::default() },
            Some(PtrType::Const) => quote! { std::ptr::null() },
            Some(PtrType::Mut) => quote! { std::ptr::null_mut() },
        };
        quote! {
            let #name: #out_ty = ::cursed::try_not_null!(
                #marshaler::from_foreign(#name),
                __exception,
                #fallback
            );
        }
    } else {
        quote! {
            let #name: #out_ty = ::cursed::try_not_null!(
                #marshaler::from_foreign(#name),
                __exception
            );
        }
    }
}

fn to_c_type(ty: &syn::Type) -> Result<syn::Type, syn::Error> {
    match &*ty {
        syn::Type::Path(..) | syn::Type::Reference(..) => {}
        syn::Type::Tuple(..) => {
            return Err(syn::Error::new(ty.span(), "Tuple parameters not supported"))
        }
        _ => return Err(syn::Error::new(ty.span(), "Unknown parameters not supported")),
    }

    let v: Option<syn::Type> =
        TYPE_MAPPING.get(&*quote! { #ty }.to_string()).and_then(|x| syn::parse_str(x).ok());

    match v {
        Some(ty) => Ok(ty),
        None => {
            let c_ty: syn::Type = syn::parse2(quote! { *const ::libc::c_void }).unwrap();
            Ok(c_ty.clone())
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

fn default_marshaler(ty: &syn::Type) -> Option<syn::Path> {
    DEFAULT_MARSHALERS.get(&*quote! { #ty }.to_string()).and_then(|x| syn::parse_str(x).ok())
}

fn is_passthrough_type(ty: &syn::Type) -> bool {
    PASSTHROUGH_TYPES.contains(&&*quote! { #ty }.to_string())
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
