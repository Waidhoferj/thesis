use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, punctuated::Punctuated, token::Comma, DeriveInput, Field, Ident, Type,
};

#[proc_macro_derive(CRDT)]
pub fn derive_crdt(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let struct_name = &ast.ident;
    let state_vector_name = format!("{struct_name}StateVector");
    let state_vector_name = syn::Ident::new(&state_vector_name, struct_name.span());
    let delta_name = format!("{struct_name}Delta");
    let delta_name = syn::Ident::new(&delta_name, struct_name.span());
    let crdt_name = format!("{struct_name}CRDT");
    let crdt_name = syn::Ident::new(&crdt_name, struct_name.span());

    let imports = quote!(
        use shelf_crdt::traits;
    );

    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = ast.data
    {
        named
    } else {
        panic!("This only works for structs.");
    };

    let state_vec_fields: Punctuated<Field, Comma> = fields
        .iter()
        .map(|field| {
            let mut sv_field = field.clone();
            sv_field.ty = Type::Verbatim(quote!(usize));
            sv_field
        })
        .collect();

    let delta_fields: Punctuated<Field, Comma> = fields
        .iter()
        .map(|field| {
            let mut delta_field = field.clone();
            let ty = delta_field.ty;
            delta_field.ty = Type::Verbatim(quote!(std::option::Option<(#ty, usize)>));
            delta_field
        })
        .collect();

    let props: Vec<Ident> = fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap().clone())
        .collect();

    let merge_components = props.iter().map(|field_name| {
        let merge = quote! {
            if let Some((val, time)) = other.#field_name {
                match self.clocks.#field_name.cmp(&time) {
                    std::cmp::Ordering::Less => {
                        self.state.#field_name = val;
                        self.clocks.#field_name = time;
                    }
                    std::cmp::Ordering::Equal if self.state.#field_name < val => {
                        self.state.#field_name = val;
                    }
                    _ => (),
                }
            }
        };
        merge
    });

    let delta_components = props.iter().map(|name| {
        quote!(
            let #name = if self.clocks.#name >= sv.#name {
                Some((self.state.#name.clone(), self.clocks.#name))
            } else {
                None
            };
        )
    });

    let update_components = props.iter().map(|name| {
        quote!(
            if self.state.#name != data.#name {
                self.clocks.#name += 1;
                self.state.#name = data.#name.clone();
            }
        )
    });

    let expanded = quote! {
        #imports

        struct #state_vector_name {
            #state_vec_fields
        }

        struct #delta_name {
            #delta_fields
        }
        struct #crdt_name {
            state: #struct_name,
            clocks: #state_vector_name,
        }

        impl traits::Mergeable for #crdt_name {
            type Other = #delta_name;
            fn merge(&mut self, other: Self::Other) {
                #(#merge_components)*
            }
        }

        impl traits::DeltaCRDT for #crdt_name {
            type Delta = #delta_name;
            type StateVector = #state_vector_name;

            fn get_state_vector(&self) -> Self::StateVector {
                self.clocks.clone()
            }

            fn get_state_delta(&self, sv: &Self::StateVector) -> Option<Self::Delta> {
                #(#delta_components)*

                Some(Self::Delta { #(#props,)* })
            }
        }

        impl #crdt_name {
            fn update(&mut self, data: &#struct_name) {
                #(#update_components)*
            }
        }

        impl Default for #state_vector_name {
            fn default() -> Self {
                Self {
                    #(#props : 0,)*
                }
            }
        }

        impl Clone for #state_vector_name {
            fn clone(&self) -> Self {
                Self {
                    #(#props : self.#props.clone(),)*
                }
            }
        }

        impl CRDTBackend for #struct_name {
            type Backend = #crdt_name;
            fn new_crdt(&self) -> Self::Backend {
                #crdt_name {
                    clocks: #state_vector_name::default(), state: self.clone(),
                }
            }
        }

    };
    expanded.into()
}
