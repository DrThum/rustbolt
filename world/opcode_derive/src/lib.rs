use proc_macro::TokenStream;
use quote::quote;

// Generate the impl ServerMessagePayload for this struct
#[proc_macro_attribute]
pub fn server_opcode(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input.clone()).unwrap();
    let name = &ast.ident;

    let gen = quote! {
        #ast

        impl ServerMessagePayload<{ Opcode::#name as u16 }> for #name {}
    };

    gen.into()
}
