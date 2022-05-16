use proc_macro::TokenStream;
use proc_macro_roids::DeriveInputExt;
use quote::quote;

use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, token::Comma, DeriveInput, Field,
    Ident, Type,
};

#[proc_macro_derive(CRDT)]
pub fn derive_crdt(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    ast.append_derives(parse_quote!(Serialize, Deserialize, Clone));
    let struct_name = &ast.ident;
    let state_vector_name = format!("{struct_name}StateVector");
    let state_vector_name = syn::Ident::new(&state_vector_name, struct_name.span());
    let delta_name = format!("{struct_name}Delta");
    let delta_name = syn::Ident::new(&delta_name, struct_name.span());
    let crdt_name = format!("{struct_name}CRDT");
    let crdt_name = syn::Ident::new(&crdt_name, struct_name.span());

    let imports = quote!(
        use shelf_crdt::traits;
        use serde;
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

    let structs: [DeriveInput; 3] = [
        parse_quote!(
        struct #state_vector_name {
            #state_vec_fields
        }),
        parse_quote!(
            struct #delta_name {
                #delta_fields
            }
        ),
        parse_quote!(
            struct #crdt_name {
                state: #struct_name,
                clocks: #state_vector_name,
            }
        ),
    ];
    let structs = structs.into_iter().map(|mut s| {
        s.append_derives(parse_quote!(
            serde::Serialize,
            serde::Deserialize,
            std::default::Default
        ));
        s
    });

    let props: Vec<Ident> = fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap().clone())
        .collect();

    let merge_delta_components = props.iter().map(|field_name| {
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
    // TODO: This will force override, fix it later. Shouldn't be a problem for non overlapping users.
    // This should be fine if we consider a merge to override previous values.
    let merge_data_components = props.iter().map(|prop| {
        quote! {
                self.clocks.#prop += 1;
                self.state.#prop = other.#prop.clone();
        }
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

    let expanded = quote! {
        #imports
        #(#structs)*

        impl traits::Mergeable<#delta_name> for #crdt_name {
            fn merge(&mut self, other:  #delta_name) {
                #(#merge_delta_components)*
            }
        }

        impl traits::Mergeable<#struct_name> for #crdt_name {
            fn merge(&mut self, other:  #struct_name) {

                #(#merge_data_components)*
            }
        }

        impl std::ops::Deref for #crdt_name {
            type Target = #struct_name;

            fn deref(&self) -> &Self::Target {
                &self.state
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

        impl Clone for #state_vector_name {
            fn clone(&self) -> Self {
                Self {
                    #(#props : self.#props.clone(),)*
                }
            }
        }

        impl traits::CRDTBackend for #struct_name {
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
