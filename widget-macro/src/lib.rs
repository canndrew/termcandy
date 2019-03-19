extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn widget(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = syn::parse_macro_input!(item as syn::ItemFn);
    /*
    let output = match func.decl.output {
        syn::ReturnType::Default => {
            let ty = (quote! { () }).into();
            syn::parse_macro_input!(ty as syn::Type)
        },
        syn::ReturnType::Type(_, ty) => *ty,
    };
    let output = (quote! { -> impl termcandy::widget::Widget<Item=#output, Error=!> }).into();
    */
    let (item, err) = match func.decl.output {
        syn::ReturnType::Default => panic!("#[widget] function must return a Result"),
        syn::ReturnType::Type(_, ty) => match *ty {
            syn::Type::Path(mut type_path) => {
                assert!(type_path.qself.is_none());
                let segment = type_path.path.segments.pop().unwrap().into_value();
                assert_eq!(segment.ident.to_string(), "Result");
                match segment.arguments {
                    syn::PathArguments::AngleBracketed(args) => {
                        let mut args = args.args;
                        assert_eq!(args.len(), 2);
                        let err = match args.pop().unwrap().into_value() {
                            syn::GenericArgument::Type(ty) => ty,
                            _ => panic!("expected type argument"),
                        };
                        let item = match args.pop().unwrap().into_value() {
                            syn::GenericArgument::Type(ty) => ty,
                            _ => panic!("expected type argument"),
                        };
                        (item, err)
                    },
                    _ => panic!("expected type parameters for Result"),
                }
            },
            _ => panic!("#[widget] function must return a Result"),
        },
    };
    let output = (quote! { -> impl termcandy::Widget<Item=#item, Error=#err> }).into();
    func.decl.output = syn::parse_macro_input!(output as syn::ReturnType);

    let block = *func.block;
    let block = quote! {{
        termcandy::widget::GenWidget::new(static move || {
            if false {
                yield (Box::new(termcandy::widget::nil_drawer) as Box<dyn Fn(termcandy::graphics::SurfaceMut) + 'static>);
            }
            #block
        })
    }}.into();
    func.block = Box::new(syn::parse_macro_input!(block as syn::Block));

    let ret = quote! { #func };
    ret.into()
}

/*
#[proc_macro]
pub fn await_widget(expr: TokenStream) -> TokenStream {
    let expr = syn::parse_macro_input!(expr as syn::Expr);
    let ret = quote! {{
        let mut widget = (#expr);
        loop {
            match widget.poll().unwrap() {
                Async::Ready(val) => break val,
                Async::NotReady => yield unsafe {
                    forge_lifetime(Box::new(|surface| widget.draw(surface)))
                },
            }
        }
    }};
    ret.into()
}
*/

