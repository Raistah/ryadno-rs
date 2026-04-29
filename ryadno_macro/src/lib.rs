use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{LitStr, Token, TypeBareFn, TypePath, parse::Parse, parse_macro_input};

struct AddClosuresRegistryArgs {
    field_type: TypePath,
    property_name: LitStr,
    fn_type: TypeBareFn,
}

impl Parse for AddClosuresRegistryArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let field_type = input.parse()?;
        _ = input.parse::<Token![,]>();
        let property_name = input.parse()?;
        _ = input.parse::<Token![,]>();
        let fn_type = input.parse()?;

        Ok(Self {
            field_type,
            property_name,
            fn_type,
        })
    }
}

impl quote::ToTokens for AddClosuresRegistryArgs {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let field_type = &self.field_type.path;
        let fn_type = &self.fn_type;
        let field_type_part = field_type
            .segments
            .iter()
            .map(|seg| {
                seg.ident
                    .to_string()
                    .replace(" ", "")
                    .replace("_", "")
                    .to_uppercase()
            })
            .collect::<Vec<String>>()
            .join("_");
        let property_name_part = self
            .property_name
            .value()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_uppercase();
        let const_name = quote::format_ident!(
            "CLOSURES_REGISTRY_{}_{}",
            field_type_part,
            property_name_part
        );

        let base_path = match crate_name("ryadno") {
            Ok(FoundCrate::Itself) => quote!(crate),
            Ok(FoundCrate::Name(name)) => {
                let ident = quote::format_ident!("{}", name);
                quote!(::#ident)
            }
            _ => quote!(::ryadno),
        };

        tokens.extend(quote! {
            #[#base_path::linkme::distributed_slice]
             pub static #const_name: [(
                 &'static str,
                  #fn_type,
             )];
        });
    }
}

#[proc_macro]
pub fn add_closures_registry(input: TokenStream) -> proc_macro::TokenStream {
    let data = parse_macro_input!(input as AddClosuresRegistryArgs);
    quote! {#data}.into()
}
