mod plugin;
use plugin::Plugin;

mod kioto;
use kioto::KiotoMetadata;
use kioto::MetadataFields;

use syn::parse_macro_input;

/// Derives reality `Plugin` trait and enables helper attribute for defining callbacks
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