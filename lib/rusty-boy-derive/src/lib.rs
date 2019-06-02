#![recursion_limit = "128"]

// Code from and/or inspired by http://nercury.github.io/rust/opengl/tutorial/2018/07/11/opengl-in-rust-from-scratch-10-procedural-macros.html

extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use syn::Token;

#[proc_macro_derive(VertexAttribPointers, attributes(location))]
pub fn vertex_attrib_pointers_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    let gen = generate_impl(ast);
    gen
}

fn generate_impl(ast: syn::DeriveInput) -> proc_macro::TokenStream {
    let ident = &ast.ident;
    let generics = &ast.generics;
    let where_clause = &ast.generics.where_clause;

    let fields_vertex_attrib_pointer = generate_vertex_attrib_pointer_calls(ast.data);
    let tok_strm2 = quote! {
        impl #ident #generics #where_clause {
            fn vertex_attrib_pointers() {
                let stride = std::mem::size_of::<Self>();
                let offset = 0;

                #(#fields_vertex_attrib_pointer)*
            }
        }
    };
    //panic!("generate impl quote: {:#?}", tok_strm2);
    tok_strm2.into()
}

fn generate_vertex_attrib_pointer_calls(body: syn::Data) -> Vec<Box<dyn quote::ToTokens>> {
    match body {
        syn::Data::Enum(_) => panic!("VertexAttribPointers can not be implemented for enums"),
        syn::Data::Union(_) => panic!("VertexAttribPointers can not be implemented for unions"),
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Unit,
            ..
        }) => panic!("VertexAttribPointers can not be implemented for Unit structs"),
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Unnamed(_),
            ..
        }) => panic!("VertexAttribPointers can not be implemented for Tuple structs"),
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
            ..
        }) => named
            .into_iter()
            .map(generate_struct_field_vertex_attrib_pointer_call)
            .collect(),
    }
}

fn generate_struct_field_vertex_attrib_pointer_call(field: syn::Field) -> Box<dyn quote::ToTokens> {
    let field_name = field.ident.map(|id| format!("{}", &id)).unwrap_or_default();
    let location_attr = field
        .attrs
        .into_iter()
        .filter(|a| a.path.segments.iter().next().unwrap().ident.to_string() == "location")
        .next()
        .unwrap_or_else(|| {
            panic!(
                "Field {:?} is missing #[location = ?] attribute",
                field_name
            )
        });

    let location_value_literal = location_attr.tts.into_iter().skip(1).next();

    let field_ty = &field.ty;
    Box::new(quote! {
         let location = #location_value_literal;
         unsafe {
             #field_ty::vertex_attrib_pointer(stride, location, offset);
         }
         let offset = offset + std::mem::size_of::<#field_ty>();
    })
}
