use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse2, Data, DeriveInput, Fields,
};

pub fn enum_from_impl(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse2(input).expect("Failed to parse input");
    let enum_name = &input.ident;

    let Data::Enum(data_enum) = &input.data else {
        return syn::Error::new_spanned(
            enum_name,
            "EnumFrom can only be used on enums",
        )
        .to_compile_error()
        .into();
    };

    let mut impls = Vec::new();
    let mut diagnostics = Vec::new();

    for variant in &data_enum.variants {
        let variant_name = &variant.ident;

        match &variant.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let ty = &fields.unnamed.first().unwrap().ty;

                impls.push(quote! {
                    impl From<#ty> for #enum_name {
                        fn from(value: #ty) -> Self {
                            #enum_name::#variant_name(value)
                        }
                    }
                });
            }

            _ => {
                let msg = format!(
                    "EnumFrom: variant `{}` does not have exactly one unnamed field",
                    variant_name
                );

                diagnostics.push(quote! {
                    compile_error!(#msg);
                });
            }
        }
    }

    quote! {
        #(#diagnostics)*
        #(#impls)*
    }
    .into()
}
