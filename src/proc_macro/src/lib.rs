//#![no_std]

use proc_macro::*;

use quote::quote;
use syn::{
    parse_macro_input, parse_quote, FnArg, Item,
    ItemFn, Path, ReturnType, Type };

/*use syn::{parse_quote_spanned, Attribute, Error, Expr, ExprLit, ExprPath, ItemStatic, ItemStruct, Lit, Visibility, ExprMacro};*/

use syn::{PatType, TypeReference, TypePath, Signature, GenericArgument};
use syn::{visit_mut::VisitMut, Stmt};


const EXPECTED_FN_ARGS: ([&'static str; 2], [&'static str; 2]) = (["ministd", "HeapRef"], ["ministd", "Allocator"]);


/*/// Defines the entry point of the kernel
/// - the function must never return (returns `!`) and must not take any parameters
#[proc_macro_attribute]
pub fn entry(attr: TokenStream, input: TokenStream) -> TokenStream {

    if !attr.is_empty() {
        panic!("Entry cannot to have any other attributes");
    }

    let mut f = parse_macro_input!(input as ItemFn);

    f.attrs.clear();

    check_signature(&f.sig);

    check_return_type(&f.sig.output);

    //  add:
    //  #[unsafe(no_mangle)]
    //  #[unsafe(export_name = "_start")]
    //  extern "C"
    f.sig.abi = Some(syn::Abi {
        extern_token: Default::default(),
        name: Some(syn::LitStr::new("sysv64", proc_macro2::Span::call_site())),
    });

    //f.attrs.push(parse_quote!(#[unsafe(no_mangle)]));
    //f.attrs.push(parse_quote!(#[unsafe(export_name = "_start")]));

    //return quote!(#f).into();

    let generated = quote! {

        //  ("testing" and "custom_testing") or not "testing"
        #[cfg(any(
            not(feature = "testing"),
            all(feature = "testing", feature = "custom_testing")
        ))]
        #[unsafe(no_mangle)]
        #[unsafe(export_name = "_start")]
        #f

        #[cfg(all(feature = "testing", not(feature = "custom_testing")))]
        #[unsafe(no_mangle)]
        extern "C" fn _start() -> ! {
            //  testing entry

            unsafe extern "Rust" {
                fn __run_tests_with(test_name: Option<&'static str>, clear: bool);
            }

            let _ = ministd::init::renderer().expect("FAILED TO INITIALIZE RENDERER");

            let _ = ministd::init::allocator().expect("FAILED TO INITIALIZE ALLOCATOR");
            
            unsafe { __run_tests_with(None, true) };
            ministd::hang();
        }

    }.into();

    return generated;







    fn check_return_type(output: &ReturnType) {
        match output {
            ReturnType::Type(_, ty) => {
                match ty.as_ref() {
                    Type::Never(_) => {
                        //  OK
                    },
                    _ => {
                        panic!("Entry function must never return (add `-> !`)");
                    }
                }
            },
            _ => {
                panic!("Entry function must never return (add `-> !`)");
            }
        }
    }

    fn check_signature(sig: &Signature) {
        if let Some(_) = sig.abi {
            panic!("Entry function cannot have any ABI set");
        }

        if let Some(_) = sig.asyncness {
            panic!("Entry function cannot be async");
        }

        if let Some(_) = sig.constness {
            panic!("Entry function cannot be constant");
        }

        if let Some(_) = sig.unsafety {
            panic!("Entry function cannot be unsafe");
        }

        if sig.inputs.len() > 0 {
            panic!("Entry function cannot take any arguments");
        }

        if let Some(_) = sig.generics.gt_token {
            panic!("Entry functioncannot have any generic arguments")
        }

        if let Some(_) = sig.variadic {
            panic!("Entry function cannot have the variadic argument");
        }
    }

}*/


#[proc_macro_attribute]
pub fn oom(attr: TokenStream, input: TokenStream) -> TokenStream {

    if !attr.is_empty() {
        panic!("oom handler cannot have any other attributes");
    }

    let mut f = parse_macro_input!(input as ItemFn);

    let sig = &f.sig;
    check_signature(sig);


    //  check first argument
    let first = sig.inputs.get(0).expect("failed to get first argument");

    if let Err(o) = check_arg(first, true, &EXPECTED_FN_ARGS.0) {
        if let Some(s) = o {
            panic!("{s}");
        } else {
            panic!("first argument must be of type &mut ministd::HeapRef");
        }
    }

    //  check second argument
    let second = sig.inputs.get(1).expect("failed to get first argument");
    if let Err(o) = check_arg(second, false, &EXPECTED_FN_ARGS.1) {
        if let Some(s) = o {
            panic!("{s}");
        } else {
            panic!("second argument must be of type &ministd::Allocator");
        }
    }

    check_return_type(&sig.output);


    //  add:
    //  #[unsafe(no_mangle)]
    //  #[unsafe(export_name = "__oom_handler")]
    //  extern "Rust"
    f.sig.abi = Some(syn::Abi {
        extern_token: Default::default(),
        name: Some(syn::LitStr::new("Rust", proc_macro2::Span::call_site())),
    });

    f.attrs.push(parse_quote!(#[unsafe(no_mangle)]));
    f.attrs.push(parse_quote!(#[unsafe(export_name = "__ministd_oom_handler")]));
    

    return quote!(#f).into();




    fn check_return_type(output: &ReturnType) {
        let path = match output {
            ReturnType::Type(_, ty) => {
                if let Type::Path(TypePath { path, .. }) = &**ty {
                    path
                } else {
                    panic!("OMM handler must never return (add -> !)")
                }
            },
            _ => panic!("OMM handler must never return (add -> !)"),
        };

        if path.segments.len() != 1 {
            panic!("OMM handler is required to return `Result<(), ()>`");
        }


        let result_segment = path.segments.last().expect("cannot get last generci argument of the returntype");
        if let syn::PathArguments::AngleBracketed(args) = &result_segment.arguments {
            let mut args = args.args.iter();

            let Some(GenericArgument::Type(Type::Tuple(ok))) = args.next() else {
                panic!("OMM handler is required to return `Result<(), ()>`");
            };
            if !ok.elems.is_empty() {
                panic!("OMM handler is required to return `Result<(), ()>`");
            }

            let Some(GenericArgument::Type(Type::Tuple(err))) = args.next() else { 
                panic!("OMM handler is required to return `Result<(), ()>`");
            };
            if !err.elems.is_empty() {
                panic!("OMM handler is required to return `Result<(), ()>`");
            }
        }
    }


    fn check_signature(sig: &Signature) {
        if let Some(_) = sig.abi {
            panic!("OOM handler cannot have any ABI set");
        }

        if let Some(_) = sig.asyncness {
            panic!("OOM handler cannot be async");
        }

        if let Some(_) = sig.constness {
            panic!("OOM handler cannot be constant");
        }

        if let Some(_) = sig.unsafety {
            panic!("OOM handler cannot be unsafe");
        }

        if sig.inputs.len() != 2 {
            panic!("OOM handler have to have exactly 2 arguments of type `&mut ministd::HeapRef` and `&ministd::AllocatorRef`");
        }

        if let Some(_) = sig.generics.gt_token {
            panic!("OOM handler cannot have any generic arguments")
        }

    }

    fn check_arg(arg: &FnArg, mutable: bool, expected: &[&'static str]) -> Result<(), Option<&'static str>> {
        let FnArg::Typed(PatType { ty , ..}) = arg else {
            return Err(None)
        };


        let path = if let Type::Reference(TypeReference {
            mutability, elem, ..}) = &**ty {
            
            if mutable != mutability.is_some() {
                if mutable {
                    panic!("make the argument a mutable reference")
                } else {
                    panic!("the argument cannot be mutable reference")
                }
            }

            let Type::Path(TypePath { path: type_path, .. }) = &**elem else {
                return Err(None)
            };
            
            type_path
        } else {
            panic!("the argument must be a mutable reference")
        };

        match path.segments.len() {
            1 => {
                let seg = path.segments.get(0).expect("unexpected internal error 1")
                    .ident.span().source_text().expect("unexpected internal error 2");
                if seg != expected[1] {
                    return Err(None)
                }
            },
            2 => {
                for (i, seg) in path.segments.iter().enumerate() {
                    let s = if let Some(s) = seg.ident.span().source_text() {
                        s
                    } else {
                        return Err(None)
                    };
                    //let s = seg.ident.span().source_text().expect("unexpected internal error 3");
                    if s != expected[i] {
                        return Err(None)
                    }

                }
            }
            _ => return Err(None)
        }

        Ok(())
    }


}
    



/*#[proc_macro_attribute]
pub fn region_finder(_: TokenStream, input: TokenStream) -> TokenStream {

    let mut f = parse_macro_input!(input as ItemFn);

    check_signature(&f.sig);

    check_return_type(&f.sig.output);

    //  add:
    //  #[unsafe(no_mangle)]
    //  #[unsafe(export_name = "__region_finder")]
    //  extern "Rust"
    f.sig.abi = Some(syn::Abi {
        extern_token: Default::default(),
        name: Some(syn::LitStr::new("Rust", proc_macro2::Span::call_site())),
    });

    f.attrs.push(parse_quote!(#[unsafe(no_mangle)]));
    f.attrs.push(parse_quote!(#[unsafe(export_name = "__region_finder")]));

    return quote!(#f).into();









    fn check_signature(sig: &Signature) {
        if let Some(_) = sig.abi {
            panic!("region finder cannot have any ABI set");
        }

        if let Some(_) = sig.asyncness {
            panic!("region finder cannot be async");
        }

        if let Some(_) = sig.constness {
            panic!("region finder cannot be constant");
        }

        if let Some(_) = sig.unsafety {
            panic!("region finder cannot be unsafe");
        }

        if sig.inputs.len() > 0 {
            panic!("region finder cannot take any arguments");
        }

        if let Some(_) = sig.generics.gt_token {
            panic!("region finder cannot have any generic arguments")
        }

        if let Some(_) = sig.variadic {
            panic!("region finder cannot have the variadic argument");
        }
    }

    fn check_return_type(output: &ReturnType) {

        //  check if output is Path type
        let path = match output {
            ReturnType::Type(_, ty) => {
                if let Type::Path(TypePath { path, .. }) = &**ty {
                    path
                } else {
                    must_return_panic();
                }
            },
            _ => must_return_panic(),
        };

        //  check if returned type is Result
        let last = path.segments.last().expect("internal failure: cannot get last");
        if last.ident != "Result" {
            must_return_panic()
        }

        
        
        //let result_segment = path.segments.last().unwrap();
        // Check for generic arguments: <ministd::mem::Region, Option<&'static str>>
        if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
            let mut args = args.args.iter();

            //  check Ok
            let Some(GenericArgument::Type(Type::Path(TypePath { path, .. }))) = args.next() else {
                panic!("region finder must return Result<ministd::mem::Region, ..> (with full path)");
            };
            if !matches_path(&path, &["ministd", "mem", "Region"]) {
                panic!("region finder must return Result<ministd::mem::Region, ..> (with full path)");
            }

            //  check Err
            let Some(GenericArgument::Type(Type::Path(TypePath { path, .. }))) = args.next() else {
                panic!("region finder must return Result<.. , Option<&'static str>");
            };

            if path.segments.len() != 1 {
                panic!("region finder must return Result<.. , Option<&'static str>")
            }

            let last = path.segments.last().expect("internal failure");

            if last.ident != "Option" {
                panic!("region finder must return Result<.. , Option<&'static str>");
            }

            //  check generic argument &'static str
            if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
                if args.args.len() != 1 {
                    must_return_panic()
                }

                match args.args.last().expect("internam failure") {
                    GenericArgument::Type(Type::Reference(TypeReference { lifetime, mutability, elem, .. })) => {
                        if mutability.is_some() {
                            panic!("region finder must return Result<.. , Option<&'static str> (immutable reference)");
                        }

                        if let Some(l) = lifetime {
                            if l.ident != "static" {
                                panic!("region finder must return Result<.. , Option<&'static str> (static lifetime)");
                            }
                        } else {
                            panic!("region finder must return Result<.. , Option<&'static str> (static lifetime)");
                        }

                        if let Type::Path(TypePath { path, .. }) = &**elem {
                            if path.segments.len() != 1 {
                                panic!("region finder must return Result<.. , Option<&'static str> (str)");
                            }
                            let last = path.segments.last().expect("failed");
                            if last.ident != "str" {
                                panic!("region finder must return Result<.. , Option<&'static str> (str)");
                            }
                        } else {
                            panic!("region finder must return Result<.. , Option<&'static str> (str)");
                        }

                        //  OK

                    },
                    _ => must_return_panic(),
                }

            } else {
                must_return_panic()
            }
        } else {
            must_return_panic()
        }


        fn must_return_panic() -> ! {
            panic!("region finder must return Result<ministd::mem::Region, Option<&'static str>>");
        }

        fn matches_path(path: &Path, expected: &[&'static str]) -> bool {

            if path.segments.len() != expected.len() || path.segments.is_empty() {
                return false;
            }

            let mut exp = expected.iter();
            for seg in path.segments.iter() {
                let e = exp.next().expect("internal error 2");
                if seg.ident != e {
                    return false;
                }
            }

            true
        }


    }


}*/


/*#[proc_macro_attribute]
pub fn testing(attr: TokenStream, input: TokenStream) -> TokenStream {

    let test_name = if attr.is_empty() {
        None
    } else {
        let a = attr.into_iter().nth(0).expect("failed to get attribte");

        let s = a.span().source_text().expect("failed to get test name");
        Some(s.leak::<'static>())
    };

    let mut f = parse_macro_input!(input as ItemFn);

    check_signature(&f.sig);

    //  make it `extern "Rust"`
    f.sig.abi = Some(syn::Abi {
        extern_token: Default::default(),
        name: Some(syn::LitStr::new("Rust", proc_macro2::Span::call_site())),
    });

    //  make it return `Result<(), ()>`
    f.sig.output = ReturnType::Type(
        syn::token::RArrow::default(),
        Box::new(syn::parse_quote!(Result<(), Option<&'static str>>)),
    );

    //  add custom macros to the function
    f.block.stmts.insert(0, syn::parse_quote! {
        #[allow(unused_macros)]
        macro_rules! fail {
            () => { return Err(None); };
            ($msg:literal) => { return Err(Some($msg)); };
        }
    });

    f.block.stmts.insert(0, syn::parse_quote! {
        #[allow(unused_macros)]
        macro_rules! success {
            () => { return Ok(()); }
        }
    });

    f.block.stmts.insert(0, syn::parse_quote! {
        #[allow(unused_macros)]
        macro_rules! __test_assert {
            ($e:expr) => {
                if !($e) {
                    return Err(Some(stringify!(assertion $e failed)));
                } 
            };
            ($e:expr, $msg:literal) => {
                if !($e) {
                    return Err(Some(stringify!(assertion $e failed: $msg)));
                }
            };
        }
    });

    f.block.stmts.insert(0, syn::parse_quote! {
        #[allow(unused_macros)]
        macro_rules! __test_assert_eq {
            ($left:expr, $right:expr) => {
                if ($left) != ($right) {
                    return Err(Some(stringify!(assertion $left == $right failed)));
                }
            };
            ($left:expr, $right:expr, $msg:literal) => {
                if ($left) != ($right) {
                    return Err(Some(stringify!(assertion $left == $right failed: $msg)));
                }
            }
        }
    });

    f.block.stmts.insert(0, syn::parse_quote! {
        #[allow(unused_macros)]
        macro_rules! __test_assert_ne {
            ($left:expr, $right:expr) => {
                if ($left) == ($right) {
                    return Err(Some(stringify!(assertion $left != $right failed)));
                }
            };
            ($left:expr, $right:expr, $msg:literal) => {
                if ($left) == ($right) {
                    return Err(Some(stringify!(assertion $left != $right failed: $msg)))
                }
            }
        }
    });
    


    //  replace `panic!`, `assert!`, etc. with custom macros
    let mut replacer = MacroReplacer;
    replacer.visit_item_fn_mut(&mut f);


    //  add return statement so the user will not have to
    f.block.stmts.push(syn::parse_quote! { success!(); });


    f.attrs.clear();
    

    //  modify function signature
    let orig_name = f.sig.ident.to_string();

    let name = format!("__test_{}_ptr_", orig_name);

    let static_name = proc_macro2::Ident::new(&name, proc_macro2::Span::call_site());
    let original_name = proc_macro2::Ident::new(&orig_name, proc_macro2::Span::call_site());

    let declaration = if let Some(tn) = test_name {
        quote! {
            static mut #static_name : ministd::Test = ministd::Test::new(Some( #tn ), #orig_name, #original_name);
        }
    } else {
        quote! {
            static mut #static_name : ministd::Test = ministd::Test::new(None, #orig_name, #original_name);
        }
    };

    let generated = quote! {

        #[cfg(feature = "testing")]
        #[unsafe(link_section = ".tests")]
        #[used]
        #declaration

        #[cfg(feature = "testing")]
        #f

    }.into();

    return generated;

    fn check_signature(sig: &Signature) {

        if let Some(_) = sig.abi {
            panic!("testing functions cannot have any ABI set");
        }

        if let Some(_) = sig.asyncness {
            panic!("testing functions cannot be async");
        }

        if let Some(_) = sig.constness {
            panic!("testing functions cannot be constant");
        }

        if let Some(_) = sig.unsafety {
            panic!("testing functions cannot be unsafe");
        }

        if sig.inputs.len() > 0 {
            panic!("testing functions cannot take any arguments");
        }

        if let Some(_) = sig.generics.gt_token {
            panic!("rtesting functions cannot have any generic arguments")
        }

        if let Some(_) = sig.variadic {
            panic!("testing functions cannot have the variadic argument");
        }

        match sig.output {
            ReturnType::Default => {
                //  OK
            },
            _ => panic!("testing functions cannot return any value"),
        }

    }

    struct MacroReplacer;

    impl VisitMut for MacroReplacer {
        fn visit_item_fn_mut(&mut self, node: &mut ItemFn) {
            let stmts = &mut node.block.stmts;
            for i in stmts.iter_mut() {
                let Stmt::Macro(mac) = i else {
                    continue
                };

                if mac.mac.path.is_ident("panic") {
                    mac.mac.path = syn::parse_quote!(fail);
                } else if mac.mac.path.is_ident("assert") {
                    mac.mac.path = syn::parse_quote!(__test_assert);
                } else if mac.mac.path.is_ident("assert_eq") {
                    mac.mac.path = syn::parse_quote!(__test_assert_eq);
                } else if mac.mac.path.is_ident("assert_ne") {
                    mac.mac.path = syn::parse_quote!(__test_assert_ne);
                }

            }
        }
    }
}

#[proc_macro_attribute]
pub fn test_only(_attr: TokenStream, input: TokenStream) -> TokenStream {

    let inp = parse_macro_input!(input as Item);

    quote! {

        #[cfg(feature = "testing")]
        #inp

    }.into()

}*/