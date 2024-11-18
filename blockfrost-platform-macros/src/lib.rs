use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

#[proc_macro_derive(ParenFormat)]
pub fn derive_paren_format(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let gen = match input.data {
        Data::Struct(data_struct) => match data_struct.fields {
            Fields::Named(fields) => {
                let field_names: Vec<_> = fields.named.iter().map(|f| f.ident.as_ref().unwrap()).collect();
                let field_access = field_names.iter().map(|field_name| {
                    quote! {
                        format!(
                            "{}",
                            self.#field_name.to_paren_string()
                        )
                    }
                });

                quote! {
                    impl #name {
                        pub fn to_paren_string(&self) -> String {
                            format!(
                                "{} ({})",
                                stringify!(#name),
                                vec![
                                    #(#field_access),*
                                ].join(" ")
                            )
                        }
                    }
                }
            }
            Fields::Unnamed(fields) => {
                let field_indices: Vec<_> = (0..fields.unnamed.len())
                    .map(syn::Index::from)
                    .collect();
                let field_access = field_indices.iter().map(|index| {
                    quote! {
                        self.#index.to_paren_string()
                    }
                });

                quote! {
                    impl #name {
                        pub fn to_paren_string(&self) -> String {
                            format!(
                                "{} ({})",
                                stringify!(#name),
                                vec![
                                    #(#field_access),*
                                ].join(" ")
                            )
                        }
                    }
                }
            }
            Fields::Unit => {
                quote! {
                    impl #name {
                        pub fn to_paren_string(&self) -> String {
                            format!("{}", stringify!(#name))
                        }
                    }
                }
            }
        },
        Data::Enum(data_enum) => {
            let variants = data_enum.variants.iter().map(|variant| {
                let variant_name = &variant.ident;

                match &variant.fields {
                    Fields::Named(fields) => {
                        let field_names: Vec<_> = fields.named.iter().map(|f| f.ident.as_ref().unwrap()).collect();
                        let field_access = field_names.iter().map(|field_name| {
                            quote! {
                                format!(
                                    "{}",
                                    #field_name.to_paren_string()
                                )
                            }
                        });

                        quote! {
                            #name::#variant_name { #(ref #field_names),* } => {
                                format!(
                                    "{} ({})",
                                    stringify!(#variant_name),
                                    vec![
                                        #(#field_access),*
                                    ].join(" ")
                                )
                            }
                        }
                    }
                    Fields::Unnamed(fields) => {
                        let field_bindings: Vec<_> = (0..fields.unnamed.len())
                            .map(|i| syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site()))
                            .collect();
                        let field_access = field_bindings.iter().map(|field_binding| {
                            quote! {
                                #field_binding.to_paren_string()
                            }
                        });

                        quote! {
                            #name::#variant_name(#(ref #field_bindings),*) => {
                                format!(
                                    "{} ({})",
                                    stringify!(#variant_name),
                                    vec![
                                        #(#field_access),*
                                    ].join(" ")
                                )
                            }
                        }
                    }
                    Fields::Unit => {
                        quote! {
                            #name::#variant_name => {
                                format!("{}", stringify!(#variant_name))
                            }
                        }
                    }
                }
            });

            quote! {
                impl #name {
                    pub fn to_paren_string(&self) -> String {
                        match self {
                            #(#variants),*
                        }
                    }
                }
            }
        }
        Data::Union(_) => panic!("ParenFormat does not support unions"),
    };

    gen.into()
}


#[proc_macro]
pub fn define_paren_format(_input: TokenStream) -> TokenStream {
    quote! {
        pub trait ParenFormat {
            fn to_paren_string(&self) -> String;
        }

        // Integers
        impl ParenFormat for i8 { fn to_paren_string(&self) -> String { self.to_string() } }
        impl ParenFormat for i16 { fn to_paren_string(&self) -> String { self.to_string() } }
        impl ParenFormat for i32 { fn to_paren_string(&self) -> String { self.to_string() } }
        impl ParenFormat for i64 { fn to_paren_string(&self) -> String { self.to_string() } }
        impl ParenFormat for i128 { fn to_paren_string(&self) -> String { self.to_string() } }
        impl ParenFormat for isize { fn to_paren_string(&self) -> String { self.to_string() } }

        // Unsigned integers
        impl ParenFormat for u8 { fn to_paren_string(&self) -> String { self.to_string() } }
        impl ParenFormat for u16 { fn to_paren_string(&self) -> String { self.to_string() } }
        impl ParenFormat for u32 { fn to_paren_string(&self) -> String { self.to_string() } }
        impl ParenFormat for u64 { fn to_paren_string(&self) -> String { self.to_string() } }
        impl ParenFormat for u128 { fn to_paren_string(&self) -> String { self.to_string() } }
        impl ParenFormat for usize { fn to_paren_string(&self) -> String { self.to_string() } }

        // Floats
        impl ParenFormat for f32 { fn to_paren_string(&self) -> String { self.to_string() } }
        impl ParenFormat for f64 { fn to_paren_string(&self) -> String { self.to_string() } }

        // Other primitives
        impl ParenFormat for bool { fn to_paren_string(&self) -> String { self.to_string() } }
        impl ParenFormat for char { fn to_paren_string(&self) -> String { self.to_string() } }

        // Common std types
        impl ParenFormat for String { 
            fn to_paren_string(&self) -> String { 
                format!("\"{}\"", self)
            }
        }
        impl ParenFormat for &str { 
            fn to_paren_string(&self) -> String { 
                format!("\"{}\"", self)
            }
        }

        // Optional values
        impl<T: ParenFormat> ParenFormat for Option<T> {
            fn to_paren_string(&self) -> String {
                match self {
                    Some(v) => format!("Some({})", v.to_paren_string()),
                    None => "None".to_string(),
                }
            }
        }
    }.into()
}