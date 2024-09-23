mod plugin;
use plugin::Plugin;

mod kioto;
use kioto::KiotoMetadata;
use kioto::MetadataFields;

use quote::quote;
use syn::parse_macro_input;
use syn::DeriveInput;

/// Derives reality `Plugin` trait and enables helper attribute for defining callbacks
#[proc_macro_derive(
    Plugin,
    attributes(
        reality
    )
)]
pub fn derive_plugin(_item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let struct_data = parse_macro_input!(_item as Plugin);
    struct_data.render().into()
}

/// Derives `Resource` trait
#[proc_macro_derive(Resource)]
pub fn derive_resource(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(item as DeriveInput);

    let ident = &derive_input.ident;
    let (impl_generics, ty_generics, where_clause) = derive_input.generics.split_for_impl();
    quote! {
        impl #impl_generics runir::Resource for #ident #ty_generics #where_clause { }
    }.into()
}

/// Derives `Repr` and `Resource` trait
#[proc_macro_derive(Repr)]
pub fn derive_repr(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(item as DeriveInput);

    let ident = &derive_input.ident;
    let (impl_generics, ty_generics, where_clause) = derive_input.generics.split_for_impl();
    quote! {
        impl #impl_generics runir::Repr for #ident #ty_generics #where_clause { }
        impl #impl_generics runir::Resource for #ident #ty_generics #where_clause { }
    }.into()
}
/// Helper macro for adding kioto metadata fields to a struct,
/// 
/// # Example Usage
/// 
/// ```rs norun
/// #[kt_metadata(loader, build)]
/// struct MyStruct {}
/// ```
/// 
/// Which adds,
/// 
/// ```rs norun
/// #[serde(rename = "-kt-build")]
/// _kt_build: Option<BuildMetadata>,
/// #[serde(rename = "-kt-loader")]
/// _kt_loader: Option<LoaderMetadata>
/// ```
/// 
/// To set the field identifier assign an ident, for example:
/// 
/// ```rs norun
/// #[kt_metadata(loader=loader, build=build)]
/// struct MyStruct {}
/// ```
/// 
#[proc_macro_attribute]
pub fn kt_metadata(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream  {
    let fields = parse_macro_input!(args as MetadataFields);
     // eprintln!("{:?}", _metadata_args);
    let mut metadata = parse_macro_input!(input as KiotoMetadata);
    metadata.fields = fields;
    metadata.render_fields().into()
}