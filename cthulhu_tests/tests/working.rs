// #![feature(param_attrs)]

// #[cthulhu::invoke]
fn foo(_a: i32) {}

// extern "C" {
//   #[cthulhu::invoke(returns = Utf8CStrMarshaler)]
//   fn some_garbage(
//     #[cthulhu::marshal(Utf8CStrMarshaler)]
//     some_str: &str
//   ) -> &str;
// }
