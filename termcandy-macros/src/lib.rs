#![recursion_limit = "128"]

extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::parse::{Parse, ParseStream};
use syn::visit_mut::{self, VisitMut};
use syn::{parse_quote, Ident, Expr};

struct AwaitVisitor;

impl VisitMut for AwaitVisitor {
    fn visit_expr_await_mut(&mut self, _: &mut syn::ExprAwait) {}
    fn visit_expr_closure_mut(&mut self, _: &mut syn::ExprClosure) {}
    fn visit_item_mut(&mut self, _: &mut syn::Item) {}

    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        visit_mut::visit_expr_mut(self, expr);
        if let Expr::Await(expr_await) = expr {
            let attrs = &*expr_await.attrs;
            let base = &*expr_await.base;
            let new_expr = parse_quote! {{
                #(#attrs)*
                {
                    let __termcandy_widget_or_future = #base;
                    termcandy::macros_impl::pin_mut!(__termcandy_widget_or_future);
                    loop {
                        match termcandy::macros_impl::Future::poll(
                            __termcandy_widget_or_future,
                            &mut *__termcandy_cx,
                        ) {
                            termcandy::macros_impl::Poll::Ready(x) => break x,
                            termcandy::macros_impl::Poll::Pending => yield {
                                let __termcandy_widget_or_future = __termcandy_widget_or_future.as_ref().get_ref();
                                unsafe {
                                    termcandy::macros_impl::forge_drawer_lifetime(
                                        Box::new(move |surface| {
                                            termcandy::macros_impl::WidgetOrFuture::widget_or_future_draw(
                                                __termcandy_widget_or_future,
                                                surface,
                                            )
                                        })
                                    )
                                }
                            },
                        }
                    }
                }
            }};
            *expr = new_expr;
        }
    }
}

#[proc_macro_attribute]
pub fn widget(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = syn::parse_macro_input!(item as syn::ItemFn);

    let lifetimes = {
        let mut lifetimes = proc_macro2::TokenStream::new();
        let params = &func.sig.generics.params;
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

    let widget_output = match func.sig.output {
        syn::ReturnType::Default => parse_quote! { () },
        syn::ReturnType::Type(_right_arrow, ty) => ty,
    };
    func.sig.output = parse_quote! { -> impl termcandy::Widget<Output = #widget_output> #lifetimes};

    let mut block = *func.block;
    AwaitVisitor.visit_block_mut(&mut block);
    func.block = parse_quote! {{
        termcandy::macros_impl::GenWidget::new({
            let __termcandy_generator = static move |__termcandy_cx: &mut termcandy::macros_impl::Context<'_>| {
                if false {
                    yield (Box::new(termcandy::macros_impl::nil_drawer) as Box<dyn for<'s, 'm> Fn(&'m mut termcandy::graphics::SurfaceMut<'s>) + 'static>);
                }
                #block
            };
            unsafe {
                termcandy::macros_impl::forge_generator_lifetime(
                    Box::pin(__termcandy_generator)
                )
            }
        })
    }};

    let ret = quote! { #func };
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
    let mut get_draw_references = Vec::with_capacity(branches.len());
    let mut draw_statements = Vec::with_capacity(branches.len());
    for i in 0..branches.len() {
        let ident = Ident::new(&format!("widget_{}", i), Span::call_site());
        let pinned_ident = Ident::new(&format!("pinned_widget_{}", i), Span::call_site());
        drops.push(quote! {
            drop(#pinned_ident);
            drop(#ident);
        });
        idents.push(ident);
    }
    let drop_all = quote! {
        #(#drops)*
    };
    for (i, branch) in branches.into_iter().enumerate() {
        let ident = &Ident::new(&format!("widget_{}", i), Span::call_site());
        let pinned_ident = &Ident::new(&format!("pinned_widget_{}", i), Span::call_site());
        let pat = branch.pat;
        let mut expr = branch.expr;
        let mut body = branch.body;
        BreakSanitizer.visit_expr_mut(&mut expr);
        BreakSanitizer.visit_expr_mut(&mut body);
        let let_binding = quote! {
            let mut #ident = #expr;
            let mut #pinned_ident = unsafe {
                termcandy::macros_impl::Pin::new_unchecked(&mut #ident)
            };
        };
        let match_expr = quote! {
            match termcandy::macros_impl::Future::poll(#pinned_ident.as_mut(), __termcandy_cx) {
                termcandy::macros_impl::Poll::Ready(#pat) => {
                    #drop_all
                    let __termcandy_select_result = #body;
                    #[allow(unreachable_code)]
                    {
                        break 'select_widget __termcandy_select_result;
                    }
                },
                termcandy::macros_impl::Poll::Pending => (),
            };
        };
        let draw_statement = quote! {
            termcandy::macros_impl::WidgetOrFuture::widget_or_future_draw(
                #ident,
                surface,
            );
        };
        let get_draw_reference = quote! {
            let #ident = #pinned_ident.as_ref().get_ref();
        };
        let_bindings.push(let_binding);
        match_exprs.push(match_expr);
        get_draw_references.push(get_draw_reference);
        draw_statements.push(draw_statement);
    }

    let inner = quote! {
        #(#let_bindings)*

        loop {
            #(#match_exprs)*

            yield {
                #(#get_draw_references)*
                unsafe {
                    termcandy::macros_impl::forge_drawer_lifetime(Box::new(move |surface| {
                        #(#draw_statements)*
                    }))
                }
            };
        }
    };

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

struct BreakSanitizer;

impl visit_mut::VisitMut for BreakSanitizer {
    fn visit_expr_async_mut(&mut self, _: &mut syn::ExprAsync) {}
    fn visit_expr_closure_mut(&mut self, _: &mut syn::ExprClosure) {}
    fn visit_expr_for_loop_mut(&mut self, _: &mut syn::ExprForLoop) {}
    fn visit_expr_loop_mut(&mut self, _: &mut syn::ExprLoop) {}
    fn visit_expr_while_mut(&mut self, _: &mut syn::ExprWhile) {}
    fn visit_item_mut(&mut self, _: &mut syn::Item) {}

    fn visit_expr_break_mut(&mut self, expr_break: &mut syn::ExprBreak) {
        if expr_break.label.is_none() {
            let expr = &expr_break.expr;
            let new_expr = parse_quote! {{
                compile_fail!("break statements that break out of select! must use labels to avoid conflicting with select!'s implementation");
                #(#expr)?
            }};
            expr_break.expr = Some(new_expr);
        }
    }
}

