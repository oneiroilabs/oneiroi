use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Expr, Fields, ItemStruct, Lit, Meta, parse_macro_input};

#[proc_macro_attribute]
pub fn graph_node(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // 1. Parse as ItemStruct (used for attribute macros) instead of DeriveInput
    let mut input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let mut doc_lines = Vec::new();

    // 2. Safely extract and modify fields in place
    if let Fields::Named(ref mut fields) = input.fields {
        for field in &mut fields.named {
            let field_ty = &field.ty;

            // Rewrite the field type in the actual struct definition
            field.ty =
                syn::parse2(quote! { ::oneiroi_core::property::Property<#field_ty> }).unwrap();

            // We must drain/remove our custom #[slot] and #[property] attributes
            // so the compiler doesn't throw "cannot find attribute in this scope" errors.
            let mut retained_attrs = Vec::new();

            for attr in field.attrs.drain(..) {
                if attr.path().is_ident("slot") {
                    // Handle slot logic if needed
                } else if attr.path().is_ident("property") {
                    // Handle property logic if needed
                } else if attr.path().is_ident("doc")
                    && let Meta::NameValue(meta) = &attr.meta
                    && let Expr::Lit(expr_lit) = &meta.value
                    && let Lit::Str(lit_str) = &expr_lit.lit
                {
                    doc_lines.push(lit_str.value().trim_start().to_string());
                } else {
                    retained_attrs.push(attr);
                }
            }
            field.attrs = retained_attrs;
        }
    } else {
        panic!("graph_node macro only supports structs with named fields");
    }

    let combined_docs = doc_lines.join("\n");

    // 3. Output the modified struct AND the new methods cleanly
    let expanded = quote! {
        // This outputs your newly modified struct definition with wrapped Property<T> types
        #input

        impl #name {
            pub fn test_doc_method() -> &'static str {
                #combined_docs
            }
        }
    };

    TokenStream::from(expanded)
}
