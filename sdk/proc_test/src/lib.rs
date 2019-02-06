extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse::Error,
    spanned::Spanned,
    parse_macro_input,
    ItemFn,
};

enum ParsedType {
    Utf8String,
    ArrayVector,
}

impl ParsedType {
    pub fn from_type(input_type: &syn::Type) -> syn::Result<ParsedType> {

        // parses generic param T in Vec<T> to string representation
        fn parse_vec_bracket(args: &syn::PathArguments, span: Span) -> syn::Result<String> {

            // checks that T is angle bracketed
            let generic_arg = match args {
                syn::PathArguments::AngleBracketed(args) => Some(args),
                _ => {
                    return Err(Error::new(span, "Invalid type in Vec brackets"));
                }
            }.unwrap();

            let arg = generic_arg.args.first().unwrap();
            let arg_val = arg.value();

            // converts T to syn::Type
            let arg_type = match arg_val {
                syn::GenericArgument::Type(ty) => Some(ty),
                _ => {
                    return Err(Error::new(span, "Invalid type in Vec brackets"))
                }
            }.unwrap();

            // converts T to syn::path
            let arg_path = match arg_type {
                syn::Type::Path(path) => Some(&path.path),
                _ => {
                    return Err(Error::new(span, "Invalid type in Vec brackets"))
                }
            }.unwrap();

            // converts T to String
            let arg_segment = arg_path.segments.first().unwrap();
            let arg_segment = arg_segment.value();
            Ok(arg_segment.ident.to_string())
        }

        let path = match input_type {
            syn::Type::Path(path) => Some(&path.path),
            _ => {
                return Err(Error::new(input_type.span(), "Unknown argument type"))
            }
        }.unwrap();

        // argument can be given in full path form: ::std::string::String
        // that why the last one used
        let type_segment = path.segments.last().unwrap();
        let type_segment = type_segment.value();

        match type_segment.ident.to_string().as_str() {
            "String" => Ok(ParsedType::Utf8String),
            "Vec" =>
                match parse_vec_bracket(&type_segment.arguments, type_segment.span()) {
                    Ok(value) => match value.as_str() {
                        "u8" => Some(Ok(ParsedType::ArrayVector)),
                        _ => {
                            return Err(Error::new(value.span(), "Only String and Vec<u8> input types are availible"))
                        }
                    }.unwrap(),
                    Err(err) => Err(err)
                }
            _ => {
                return Err(Error::new(
                    type_segment.span(),
                    "Only String and Vec<u8> input types are availible"
                ))
            }
        }
    }

    pub fn from_fn_arg(fn_arg: &syn::FnArg) -> syn::Result<ParsedType> {
        let fn_arg = match fn_arg {
            syn::FnArg::Captured(arg) => Some(&arg.ty),
            _ => {
                return Err(Error::new(fn_arg.span(), "Unknown argument"))
            }
        }.unwrap();

        ParsedType::from_type(fn_arg)
    }

    pub fn from_return_type(ret_type: &syn::ReturnType) -> syn::Result<ParsedType> {
        let ret_type = match ret_type {
            syn::ReturnType::Type(_, t) => Some(t),
            _ => {
                return Err(Error::new(ret_type.span(), "Unknown argument"))
            }
        }.unwrap();

        ParsedType::from_type(ret_type.as_ref())
    }
}

trait InputTypeGenerator {
    fn generate_fn_prolog(&self) -> proc_macro2::TokenStream;
}

trait ReturnTypeGenerator {
    fn generate_fn_epilog(&self) -> proc_macro2::TokenStream;
}

impl InputTypeGenerator for ParsedType {
    fn generate_fn_prolog(&self) -> proc_macro2::TokenStream {
        match self {
            ParsedType::Utf8String =>
                quote! {
                    let arg = fluence::memory::read_input_from_mem(ptr, len);
                    let arg = String::from_utf8(arg).unwrap();
                },
            ParsedType::ArrayVector => quote! {
                    let arg = fluence::memory::read_input_from_mem(ptr, len);
            },
        }
    }
}

impl ReturnTypeGenerator for ParsedType {
    fn generate_fn_epilog(&self) -> proc_macro2::TokenStream {
        match self {
            ParsedType::Utf8String => quote! {
                fluence::memory::write_result_to_mem(
                        result.as_bytes()
                    )
                    .expect("Putting result string to memory was failed.")
                },
            ParsedType::ArrayVector => quote! {
                fluence::memory::write_result_to_mem(&result[..])
                    .expect("Putting result vector to memory was failed.")
            },
        }
    }
}

fn invoke_handler_impl(fn_item: &syn::ItemFn) -> proc_macro2::TokenStream {
    let ItemFn {
        constness,
        unsafety,
        abi,
        ident,
        decl,
        ..
    } = fn_item;

    if let Err(e) = (|| {
        if decl.inputs.len() != 1 {
            return Err(Error::new(
                decl.paren_token.span,
                "the principal module invocation handler has to have one input param",
            ));
        }
        if let Some(constness) = constness {
            return Err(Error::new(
                constness.span,
                "the principal module invocation handler has to don't be const"
            ));
        }
        if let Some(unsafety) = unsafety {
            return Err(Error::new(
                unsafety.span,
                "the principal module invocation handler has to don't be unsafe"
            ));
        }
        if let Some(abi) = abi {
            return Err(Error::new(
                abi.extern_token.span,
                "the principal module invocation handler has to don't have custom linkage",
            ));
        }
        if !decl.generics.params.is_empty() || decl.generics.where_clause.is_some() {
            return Err(Error::new(
                decl.fn_token.span,
                "the principal module invocation handler has to don't have generic params",
            ));
        }
        if let Some(variadic) = decl.variadic {
            return Err(Error::new(
                variadic.spans[0],
                "the principal module invocation handler has to don't be variadic"));
        }
        Ok(())
    })() {
        return e.to_compile_error().into();
    }

    let input_type = ParsedType::from_fn_arg(
        decl.inputs.first().unwrap().into_value()
        ).unwrap();
    let output_type = ParsedType::from_return_type(&decl.output).unwrap();

    let prolog = input_type.generate_fn_prolog();
    let epilog = output_type.generate_fn_epilog();

    let resulted_invoke = quote! {
        #fn_item

        #[no_mangle]
        pub unsafe fn invoke(ptr: *mut u8, len: usize) -> std::ptr::NonNull<u8> {
            #prolog

            let result = #ident(arg);

            #epilog
        }
    };

    resulted_invoke
}

#[proc_macro_attribute]
pub fn invoke_handler(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let fn_item = parse_macro_input!(input as ItemFn);
    invoke_handler_impl(&fn_item).into()
}
