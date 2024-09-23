use proc_macro2::Span;
use quote::{quote, quote_spanned};
use syn::{
    parse::{Parse, Parser},
    punctuated::Punctuated,
    DeriveInput, Ident, LitStr, Token,
};

pub struct KiotoMetadata {
    input: DeriveInput,
    pub fields: MetadataFields,
}

impl KiotoMetadata {
    /// Render metadata fields for the struct
    pub fn render_fields(self) -> proc_macro2::TokenStream {
        let mut ast = self.input;
        match &mut ast.data {
            syn::Data::Struct(ref mut struct_data) => match &mut struct_data.fields {
                syn::Fields::Named(fields) => {
                    for field in self.fields.fields.iter() {
                        fields.named.push(
                            field
                                .to_field()
                                .expect("should be able to convert to field"),
                        );
                    }
                }
                _ => (),
            },
            _ => panic!("`add_field` has to be used with structs "),
        }

        let name = &ast.ident;
        let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
        let overrides = self.fields.render_metadata_trait();
        quote! {
            #ast

            impl #impl_generics kioto::engine::Metadata for #name #ty_generics #where_clause {
                #overrides
            }
        }
    }
}

impl Parse for KiotoMetadata {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let input = DeriveInput::parse(input)?;
        Ok(Self {
            input,
            fields: MetadataFields::default(),
        })
    }
}

#[derive(Default)]
pub struct MetadataFields {
    fields: Vec<MetadataOptions>,
}

impl MetadataFields {
    fn render_metadata_trait(&self) -> proc_macro2::TokenStream {
        let render_overrides = self.fields.iter().map(|f| f.render_metdata_trait_override());
        quote! {
            #(#render_overrides)*
        }
    }
}

impl Parse for MetadataFields {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let fields = Punctuated::<MetadataOptions, Token![,]>::parse_terminated(input)?;
        Ok(Self {
            fields: fields.iter().cloned().collect(),
        })
    }
}

#[derive(Clone)]
enum MetadataTypes {
    Build,
    Loader,
}

impl MetadataTypes {
    fn split_for_render(&self, span: Span) -> (syn::Ident, syn::LitStr, proc_macro2::TokenStream) {
        match self {
            MetadataTypes::Build => (
                Ident::new("_kt_build", span),
                LitStr::new("-kt-build", span),
                quote! { Option<kioto::engine::BuildMetadata> },
            ),
            MetadataTypes::Loader => (
                Ident::new("_kt_loader", span),
                LitStr::new("-kt-loader", span),
                quote! { Option<kioto::engine::LoaderMetadata> },
            ),
        }
    }
}

#[derive(Clone)]
enum MetadataOptions {
    Default(Span, MetadataTypes),
    FieldName(Span, MetadataTypes, syn::Ident),
}

impl Parse for MetadataOptions {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<Ident>()?;
        let meta_ty = match ident.to_string().as_str() {
            "build" => MetadataTypes::Build,
            "loader" => MetadataTypes::Loader,
            _ => {
                return Err(syn::Error::new(
                    input.span(),
                    "Only supported metadata types are `build` or `loader`",
                ))
            }
        };

        if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            Ok(MetadataOptions::FieldName(
                input.span(),
                meta_ty,
                input.parse::<Ident>()?,
            ))
        } else {
            Ok(MetadataOptions::Default(input.span(), meta_ty))
        }
    }
}

impl MetadataOptions {
    fn render_metdata_trait_override(&self) -> proc_macro2::TokenStream {
        match self {
            MetadataOptions::Default(span, metadata_types) => match metadata_types {
                MetadataTypes::Build => {
                    quote_spanned! {span.clone()=>
                        fn build(&self) -> Option<&kioto::engine::BuildMetadata> {
                            self._kt_build.as_ref()
                        }
                    }
                }
                MetadataTypes::Loader => {
                    quote_spanned! {span.clone()=>
                        fn loader(&self) -> Option<&kioto::engine::LoaderMetadata> {
                            self._kt_loader.as_ref()
                        }
                    }
                },
            },
            MetadataOptions::FieldName(span, metadata_types, ident) => match metadata_types {
                MetadataTypes::Build => {
                    let ident = &ident;
                    quote_spanned! {span.clone()=>
                        fn build(&self) -> Option<&kioto::engine::BuildMetadata> {
                            self.#ident.as_ref()
                        }
                    }
                }
                MetadataTypes::Loader => {
                    let ident = &ident;
                    quote_spanned! {span.clone()=>
                        fn loader(&self) -> Option<&kioto::engine::LoaderMetadata> {
                            self.#ident.as_ref()
                        }
                    }
                },
            },
        }
    }

    fn to_field(&self) -> syn::Result<syn::Field> {
        let (ident, serde_rename, ty) = match self {
            MetadataOptions::Default(span, metadata_types) => {
                metadata_types.split_for_render(*span)
            }
            MetadataOptions::FieldName(span, metadata_types, ident) => {
                let (_, serde_rename, ty) = metadata_types.split_for_render(*span);
                (ident.clone(), serde_rename, ty)
            }
        };

        syn::Field::parse_named.parse2(quote! {
            #[serde(rename = #serde_rename)]
            pub #ident: #ty
        })
    }
}
