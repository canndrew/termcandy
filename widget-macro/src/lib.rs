#![recursion_limit = "128"]

extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::parse::{Parse, ParseStream};
use syn::Ident;

#[proc_macro_attribute]
pub fn widget(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = syn::parse_macro_input!(item as syn::ItemFn);

    let lifetimes = {
        let mut lifetimes = proc_macro2::TokenStream::new();
        let params = &func.decl.generics.params;
        for param in params {
            match param {
                syn::GenericParam::Lifetime(lifetime_def) => {
                    let lifetime = &lifetime_def.lifetime;
                    lifetimes.extend(quote! { + #lifetime });
                },
                _ => (),
            }
        }
        lifetimes
    };

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
    let output = (quote! { -> impl termcandy::Widget<Item=#item, Error=#err> #lifetimes}).into();
    func.decl.output = syn::parse_macro_input!(output as syn::ReturnType);

    let block = *func.block;
    let block = quote! {{
        termcandy::widget::GenWidget::new(static move || {
            if false {
                yield (Box::new(termcandy::widget::nil_drawer) as Box<dyn for<'s, 'm> Fn(&'m mut termcandy::graphics::SurfaceMut<'s>) + 'static>);
            }
            #block
        })
    }}.into();
    func.block = Box::new(syn::parse_macro_input!(block as syn::Block));

    let ret = quote! { #func };
    ret.into()
}

#[proc_macro]
pub fn await_widget(expr: TokenStream) -> TokenStream {
    let expr = syn::parse_macro_input!(expr as syn::Expr);
    let ret = quote! {{
        use futures::Future;
        use termcandy::Widget;

        let mut widget = (#expr);
        loop {
            match widget.poll()? {
                futures::Async::Ready(val) => break val,
                futures::Async::NotReady => yield unsafe {
                    termcandy::widget::forge_lifetime(Box::new(|surface| widget.draw(surface)))
                },
            }
        }
    }};
    ret.into()
}

#[proc_macro]
pub fn select_widget(branches: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(branches as SelectWidget);
    let branches: Vec<_> = parsed.punctuated.into_iter().collect();

    //let mut sanitizer = BreakSanitizer { found: false };
    let mut idents = Vec::with_capacity(branches.len());
    let mut drops = Vec::with_capacity(branches.len());
    let mut let_bindings = Vec::with_capacity(branches.len());
    let mut match_exprs = Vec::with_capacity(branches.len());
    let mut draw_statements = Vec::with_capacity(branches.len());
    for i in 0..branches.len() {
        let ident = Ident::new(&format!("widget_{}", i), Span::call_site());
        drops.push(quote! {
            drop(#ident);
        });
        idents.push(ident);
    }
    let drop_all = quote! {
        #(#drops)*
    };
    for (i, branch) in branches.into_iter().enumerate() {
        let ident = &Ident::new(&format!("widget_{}", i), Span::call_site());
        let pat = branch.pat;
        let expr = branch.expr;
        let body = branch.body;
        //syn::visit_mut::visit_expr_mut(&mut sanitizer, &mut body);
        let let_binding = quote! {
            let mut #ident = #expr;
        };
        let match_expr = quote! {
            match #ident.poll()? {
                futures::Async::Ready(#pat) => {
                    #drop_all
                    break 'select_widget (#body)
                },
                futures::Async::NotReady => (),
            };
        };
        let draw_statement = quote! {
            #ident.select_draw(surface);
        };
        let_bindings.push(let_binding);
        match_exprs.push(match_expr);
        draw_statements.push(draw_statement);
    }

    let inner = quote! {
        #(#let_bindings)*

        loop {
            #(#match_exprs)*

            yield unsafe {
                termcandy::widget::forge_lifetime(Box::new(|surface| {
                    use termcandy::widget::SelectDraw;
                    #(#draw_statements)*
                }))
            }
        }
    };

    /*
    let ret = if sanitizer.found {
        quote! { 'select_widget: {
            let break_opt = 'select_widget_sanitized: { #inner };
            match break_opt {
                Some(a) => break a,
                None => continue,
            }
        }}
    } else {
        quote! { 'select_widget: { #inner }}
    };
    */
    let ret = quote! { 'select_widget: { #inner }};
    TokenStream::from(ret)
}

struct SelectWidget {
    punctuated: Punctuated<SelectWidgetBranch, syn::Token![,]>,
}

impl Parse for SelectWidget {
    fn parse(input: ParseStream) -> syn::parse::Result<SelectWidget> {
        let punctuated = Punctuated::parse_terminated(input)?;
        Ok(SelectWidget { punctuated })
    }
}

struct SelectWidgetBranch {
    pat: syn::Pat,
    expr: syn::Expr,
    body: syn::Expr,
}

impl Parse for SelectWidgetBranch {
    fn parse(input: ParseStream) -> syn::parse::Result<SelectWidgetBranch> {
        let pat = input.parse()?;
        input.parse::<syn::Token![=]>()?;
        let expr = input.parse()?;
        input.parse::<syn::Token![=>]>()?;
        let body = input.parse()?;
        Ok(SelectWidgetBranch { pat, expr, body })
    }
}

/*
struct BreakSanitizer {
    found: bool,
}

impl syn::visit_mut::VisitMut for BreakSanitizer {
    fn visit_expr_mut(&mut self, expr: &mut syn::Expr) {
        syn::visit_mut::visit_expr_mut(self, expr);
        *expr = match expr {
            syn::Expr::Break(expr_break) => {
                if expr_break.label.is_none() {
                    let expr: proc_macro2::TokenStream = match expr_break.expr {
                        Some(ref expr) => quote! { Some(#expr) },
                        None => quote! { Some(()) },
                    };
                    let expr = TokenStream::from(expr);
                    let expr = syn::parse_macro_input::parse(expr).unwrap();
                    syn::Expr::Break(syn::ExprBreak {
                        attrs: expr_break.attrs.clone(),
                        break_token: expr_break.break_token.clone(),
                        label: Some(syn::Lifetime::new("'select_widget_sanitized", Span::call_site())),
                        expr: Some(Box::new(expr)),
                    })
                } else {
                    return;
                }
            },
            syn::Expr::Continue(expr_continue) => {
                if expr_continue.label.is_none() {
                    let expr = quote! { None };
                    let expr = expr.into();
                    let expr = syn::parse_macro_input::parse(expr).unwrap();
                    syn::Expr::Break(syn::ExprBreak {
                        attrs: expr_continue.attrs.clone(),
                        break_token: syn::token::Break { span: Span::call_site() },
                        label: Some(syn::Lifetime::new("'select_widget_sanitized", Span::call_site())),
                        expr: Some(Box::new(expr)),
                    })
                } else {
                    return;
                }
            },
            _ => return,
        };
        self.found = true;
    }
}
*/

