use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Expr, Fields, Lit, Meta, ExprLit, parse2};

pub fn enum_kind_impl(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse2(input).expect("Failed to parse input");
    let name = input.ident;

    let Data::Enum(data_enum) = input.data else {
        return syn::Error::new_spanned(
            name,
            "EnumKind can only be derived for enums"
        )
        .to_compile_error()
        .into();
    };

    let mut arms = Vec::new();
    let mut last_index: Option<usize> = None;

    for variant in data_enum.variants.iter() {
        let v_ident = &variant.ident;

        // Extract #[kind = N] if present
        let mut explicit_index = None;

        for attr in &variant.attrs {
            if attr.path().is_ident("kind") {
                match &attr.meta {
                    Meta::NameValue(nv) => {
                        match &nv.value {
                            Expr::Lit(ExprLit { lit: Lit::Int(lit_int), .. }) => {
                                let n = lit_int
                                    .base10_parse::<usize>()
                                    .unwrap();
                                
                                explicit_index = Some(n);
                            }
                            _ => {
                                return syn::Error::new_spanned(&nv.value, "Expected integer literal for #[kind = N]")
                                    .to_compile_error()
                                    .into();
                            }
                        }
                    }
                    _ => {
                        return syn::Error::new_spanned(
                            attr,
                            "Expected #[kind = N]"
                        )
                        .to_compile_error()
                        .into();
                    }
                }
            }
        }

        // Determine final index
        let index = match explicit_index {
            Some(n) => n,
            None => match last_index {
                Some(prev) => prev + 1,
                None => 0,
            },
        };

        last_index = Some(index);

        // Pattern for matching the variant
        let pat = match &variant.fields {
            Fields::Unit => quote! { #name::#v_ident },
            Fields::Unnamed(_) => quote! { #name::#v_ident (..) },
            Fields::Named(_) => quote! { #name::#v_ident { .. } },
        };

        arms.push(quote! {
            #pat => #index,
        });
    }

    let expanded = quote! {
        impl #name {
            pub fn identifier(&self) -> usize {
                match self {
                    #(#arms)*
                }
            }
        }
    };

    expanded.into()
}

#[cfg(test)]
mod tests {
    use assert_tokens_eq::assert_tokens_eq;
    use quote::quote;

    use crate::enum_kind::enum_kind_impl;

    #[test]
    pub fn valid_codegen() {
        let output = enum_kind_impl(quote! {
            #[derive(EnumKind)]
            enum Security {
                A,              // 0
                B,              // 1
                #[kind = 10]
                C,              // 10
                D,              // 11
            }
        });

        let expected = quote! {
            impl Security {
                pub fn identifier(&self) -> usize {
                    match self {
                        Security::A => 0usize,
                        Security::B => 1usize,
                        Security::C => 10usize,
                        Security::D => 11usize,
                    }
                }
            }
        };
        assert_tokens_eq!(expected, output);
    }
}