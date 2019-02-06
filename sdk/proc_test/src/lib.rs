extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::Error,
    parse_macro_input,
    ItemFn,
    LitStr,
};

enum ParsedType {
 String,
 Vec,
}

fn get_input_type(fn_item :&ItemFn) -> syn::Result<ParsedType> {
    let ItemFn {
        decl,
        ..
    } = fn_item;

    let arg_type = &decl.inputs.first().unwrap().into_value();

    let arg_type = match arg_type {
        syn::FnArg::Captured(arg) => Some(&arg.ty),
        _ => {
            return Err(Error::new(
                decl.paren_token.span,
                "unknown argument"
            ));
        }
    }.unwrap();

    let path = match arg_type {
        syn::Type::Path(path) => &path.path,
        _ => panic!("Invalid output type 1"),
    };

    println!("{}", path.segments.len());
    let vec_segment = path.segments.last().unwrap();
    let vec_segment = vec_segment.value();
    println!("{}", vec_segment.ident);
    let arguments = &vec_segment.arguments;
    let generic_arg = match arguments {
        syn::PathArguments::AngleBracketed(args) => args,
        _ => panic!("Invalid output type 2"),
    };

    let arg = generic_arg.args.first().unwrap();
    let arg_val = arg.value();
    let uu = match arg_val {
        syn::GenericArgument::Type(ty) => ty,
        _ => panic!("Invalid output type 3"),
    };

    let path = match uu {
        syn::Type::Path(path) => &path.path,
        _ => panic!("Invalid output type 4"),
    };

    let vec_segment = path.segments.first().unwrap();
    let vec_segment = vec_segment.value();
    println!("{}", vec_segment.ident);
    let arguments = &vec_segment.arguments;
    let generic_arg = match arguments {
        syn::PathArguments::AngleBracketed(args) => args,
        _ => panic!("Invalid output type 5"),
    };

    Ok(ParsedType::String)
}

fn check_fn_sginature(fn_item :&ItemFn) -> TokenStream {
    let ItemFn {
        vis,
        constness,
        unsafety,
        abi,
        ident,
        decl,
        block,
        ..
    } = fn_item;

    if let Err(e) = (|| {
        if decl.inputs.len() != 1 {
            return Err(Error::new(
                decl.paren_token.span,
                "the principal module invocation handler has to have one input param",
            ));
        }
        /*        if decl.inputs.first().unwrap().value().get_type_id() != decl.output.get_type_id()get_type_id() {
                    return Err(Error::new(
                        decl.inputs.span,
                        "the principal module invocation handler has to have the same types of input and output",
                    ));
                }
        */
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
}

#[proc_macro_attribute]
pub fn invoke_handler(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let original_fn = parse_macro_input!(input as ItemFn);
    let original_fn_clone = original_fn.clone();


    let resulted_invoke = quote! {

        #original_fn_clone

        #[no_mangle]
        pub unsafe fn invoke(ptr: *mut u8, len: usize) -> usize
            let param = fluence::memory::read_input_from_mem(ptr, len);
    };

    resulted_invoke.into()
}
