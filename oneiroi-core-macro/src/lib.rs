use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Meta, parse_macro_input};

// We declare 'slot' and 'property' as helper attributes
#[proc_macro_derive(GraphNode, attributes(slot, property))]
pub fn derive_node_variables(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let fields = match input.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named,
            _ => panic!("NodeNode macro only supports structs with named fields"),
        },
        _ => panic!("NodeNode macro only supports structs"),
    };

    let definitions = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap().to_string();

        // Default fallback if no attribute is specified
        let mut kind = quote! { VariableKind::Property };

        // Check the attributes on this specific field
        for attr in &field.attrs {
            if attr.path().is_ident("slot") {
                kind = quote! { VariableKind::Slot };
                break;
            } else if attr.path().is_ident("property") {
                kind = quote! { VariableKind::Property };
                break;
            }
        }

        quote! {
            (#field_name, #kind)
        }
    });

    let expanded = quote! {

        impl<Ctx: ::oneiroi_core::arena::ExecutionContext> ::oneiroi_core::node::Node<Ctx> for #name {
            
        }
    };

    TokenStream::from(expanded)
}
