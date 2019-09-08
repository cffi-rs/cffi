use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use quote::quote;
use syn::Type;

macro_rules! map_types {
    [$($rust:ty => $c:ty,)*] => {{
        let mut map = std::collections::HashMap::<Type, Type>::new();
        $(map.insert(
            syn::parse2(quote!{ $rust })
                .expect(concat!("cannot parse", stringify!($rust), "as type")),

            syn::parse2(quote!{ $c })
                .expect(concat!("cannot parse", stringify!($c), "as type")),
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

macro_rules! type_array {
    [$($rust:ty,)*] => {{
        vec![
            $(syn::parse2(quote!{ $rust })
                .expect(concat!("cannot parse", stringify!($rust), "as type")),
            )*
        ]
    }}
}

fn main() {
    let default_marshalers: HashMap<Type, syn::Path> = map_marshalers![
        bool => ::cursed::BoolMarshaler,
        Cow<str> => ::cursed::StrMarshaler,
        Arc<str> => ::cursed::ArcMarshaler::<str>,
        Arc<T> => ::cursed::ArcMarshaler<T>,
        Box<T> => ::cursed::BoxMarshaler<T>,
    ];

    let type_mapping: std::collections::HashMap<Type, Type> = map_types![
        bool => u8,
        u8 => ::libc::c_uchar,
        i8 => ::libc::c_char,
        i16 => ::libc::c_short,
        u16 => ::libc::c_ushort,
        i32 => ::libc::c_int,
        u32 => ::libc::c_uint,
        i64 => ::libc::c_long,
        u64 => ::libc::c_ulong,
        &'a str => *const ::libc::c_char,
        &'a CStr => *const ::libc::c_char,
        CString => *mut ::libc::c_char,
        Arc<str> => *const ::libc::c_char,
        Cow<str> => *const ::libc::c_char,
    ];

    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("codegen.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());

    write!(&mut file, "static DEFAULT_MARSHALERS: phf::Map<&'static str, &'static str> = ")
        .unwrap();
    let mut map = phf_codegen::Map::new();
    for (key, value) in default_marshalers.iter() {
        map.entry(quote! { #key }.to_string(), &format!("\"{}\"", quote! { #value }.to_string()));
    }
    map.build(&mut file).unwrap();
    write!(&mut file, ";\n").unwrap();

    write!(&mut file, "static TYPE_MAPPING: phf::Map<&'static str, &'static str> = ").unwrap();
    let mut map = phf_codegen::Map::new();
    for (key, value) in type_mapping.iter() {
        map.entry(quote! { #key }.to_string(), &format!("\"{}\"", quote! { #value }.to_string()));
    }
    map.build(&mut file).unwrap();
    write!(&mut file, ";\n").unwrap();

    let types: Vec<Type> = type_array![u8, i8, u16, i16, u32, i32, i64, u64,];

    write!(
        &mut file,
        "static PASSTHROUGH_TYPES: &[&str] = &[\"{}\"];\n",
        types.into_iter().map(|x| quote! { #x }.to_string()).collect::<Vec<_>>().join("\", \"")
    )
    .unwrap();
}
