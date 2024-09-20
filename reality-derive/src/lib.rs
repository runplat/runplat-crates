mod plugin;
use plugin::Plugin;

use syn::parse_macro_input;

/// Derives Reality object includes several implementations,
///
#[proc_macro_derive(
    Plugin,
    attributes(
        reality
    )
)]
pub fn derive_object_type(_item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let struct_data = parse_macro_input!(_item as Plugin);
    struct_data.render().into()
}