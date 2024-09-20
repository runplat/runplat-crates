use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parse;
use syn::{DeriveInput, ExprClosure, Path, Token};

/// Parser struct for implementing basics of a plugin
pub struct Plugin {
    input: DeriveInput,
    call: Option<Path>,
    content_from: Option<Path>,
    content_with: Option<Path>,
    load: Option<Path>,
    load_with: Option<ExprClosure>,
}

impl Plugin {
    fn render_content_state_uuid_impl(&self) -> TokenStream {
        if let Some(content_with) = self.content_with.as_ref() {
            quote! {
                #content_with(self)
            }
        } else if let Some(content) = self.content_from.as_ref() {
            quote! {
                #content::from(self).state_uuid()
            }
        } else {
            quote! {
                compile_error!(r#"
`content_from` or `content_with` attribute is required to derive `Plugin`
# Examples
```
#[reality(
  content_from = BincodeContent
)]
#[reality(
  content_from = NilContent
)]
#[reality(
  content_from = RandomContent
)]
#[reality(
  content_with = function_that_generates_content
)]
```
"#)
            }
        }
    }

    fn render_plugin_call_impl(&self) -> TokenStream {
        if let Some(call) = self.call.as_ref() {
            quote! {
                #call(binding)
            }
        } else {
            quote! {
                binding.skip()
            }
        }
    }

    fn render_plugin_load_impl(&self) -> TokenStream {
        if let Some(load) = self.load.as_ref() {
            quote! {
                #load(put)
            }
        } else if let Some(load_with) = self.load_with.as_ref() {
            let ident = &self.input.ident;
            quote! {
                fn _load_with(p: impl Fn(runir::store::Put<'_, #ident>) -> runir::store::Put<'_, #ident>, put: runir::store::Put<'_, #ident>) -> runir::store::Put<'_, #ident> {
                    p(put)
                }
                _load_with(#load_with, put)
            }
        } else {
            quote! {
                put
            }
        }
    }

    pub fn render(self) -> TokenStream {
        let name = &self.input.ident;
        let (impl_generic, ty_generic, where_clause) = self.input.generics.split_for_impl();
        let impl_plugin_call = self.render_plugin_call_impl();
        let impl_plugin_load = self.render_plugin_load_impl();
        let impl_content_state_uuid = self.render_content_state_uuid_impl();
        quote! {
            impl #impl_generic runir::Resource for #name #ty_generic #where_clause {}
            impl #impl_generic runir::Content for #name #ty_generic #where_clause {
                fn state_uuid(&self) -> uuid::Uuid {
                    #impl_content_state_uuid
                }
            }
            impl #impl_generic Plugin for #name #ty_generic #where_clause {
                fn call(binding: plugin::Bind<Self>) -> CallResult {
                    #impl_plugin_call
                }

                fn version() -> Version {
                    env!("CARGO_PKG_VERSION").parse().expect("should parse because cargo would complain first")
                }

                fn load(put: runir::store::Put<'_, Self>) -> runir::store::Put<'_, Self> {
                    #impl_plugin_load
                }
            }
        }
    }
}

impl Parse for Plugin {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let input = DeriveInput::parse(input)?;

        let mut call = None;
        let mut content_from = None;
        let mut content_with = None;
        let mut load = None;
        let mut load_with = None;
        for attr in input.attrs.iter() {
            if attr.path().is_ident("reality") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("call") {
                        meta.input.parse::<Token![=]>()?;
                        call = Some(meta.input.parse::<Path>()?);
                    }

                    if meta.path.is_ident("content_from") {
                        meta.input.parse::<Token![=]>()?;
                        content_from = Some(meta.input.parse::<Path>()?);
                    }

                    if meta.path.is_ident("content_with") {
                        meta.input.parse::<Token![=]>()?;
                        content_with = Some(meta.input.parse::<Path>()?);
                    }

                    if meta.path.is_ident("load") {
                        meta.input.parse::<Token![=]>()?;
                        if meta.input.peek(Token![|]) {
                            load_with = Some(meta.input.parse::<ExprClosure>()?);
                        } else {
                            load = Some(meta.input.parse::<Path>()?);
                        }
                    }
                    Ok(())
                })?;
            }
        }

        Ok(Plugin {
            input,
            call,
            content_from,
            content_with,
            load,
            load_with,
        })
    }
}
