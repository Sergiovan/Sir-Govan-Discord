extern crate proc_macro;

use proc_macro_error::{proc_macro_error, Diagnostic};
use quote::quote;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::{bracketed, token, Token};

struct CommandArguments {
	aliases: Option<Vec<syn::LitStr>>,
}

impl Parse for CommandArguments {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		if input.is_empty() {
			Ok(CommandArguments { aliases: None })
		} else {
			let param_name = input.parse::<syn::Ident>()?;
			if param_name != "aliases" {
				return Err(syn::Error::new(
					param_name.span(),
					format!(
						"Invalid parameter name {}, accepted are: 'aliases'",
						param_name
					),
				));
			}

			input.parse::<syn::Token![=]>()?;
			let content;
			let _: token::Bracket = bracketed!(content in input);

			let res = Punctuated::<syn::LitStr, Token![,]>::parse_terminated(&content)?;

			if !input.is_empty() {
				return Err(input.error("No more parameters allowed"));
			}
			assert!(input.is_empty(), "No more parameters allowed: {}", input);

			Ok(CommandArguments {
				aliases: Some(res.into_iter().collect::<Vec<_>>()),
			})
		}
	}
}

// From https://stackoverflow.com/a/71482073
fn transform_params(params: Punctuated<syn::FnArg, syn::token::Comma>) -> syn::Expr {
	// 1. Filter the params, so that only typed arguments remain
	// 2. Extract the ident (in case the pattern type is ident)
	let idents = params.iter().filter_map(|param| {
		if let syn::FnArg::Typed(pat_type) = param {
			if let syn::Pat::Ident(pat_ident) = *pat_type.pat.clone() {
				return Some(pat_ident.ident);
			}
		}
		None
	});

	// Add all idents to a Punctuated => param1, param2, ...
	let mut punctuated: Punctuated<syn::Ident, token::Comma> = Punctuated::new();
	idents.for_each(|ident| punctuated.push(ident));

	// Generate expression from Punctuated (and wrap with parentheses)
	let transformed_params = syn::parse_quote!((#punctuated));
	transformed_params
}

// TODO Eventually maybe add type checking?
#[proc_macro_error]
#[proc_macro_attribute]
pub fn command(
	args: proc_macro::TokenStream,
	item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let args = syn::parse_macro_input!(args as CommandArguments);
	let input = syn::parse_macro_input!(item as syn::ItemFn);

	if input.sig.asyncness.is_none() {
		Diagnostic::spanned(
			input.sig.ident.span(),
			proc_macro_error::Level::Error,
			"Function must be async".to_string(),
		)
		.emit();
		return quote! { #input }.into();
	}

	let function_name_str = input.sig.ident.to_string();

	let mut name_chars = function_name_str.chars();
	let type_name = match name_chars.next() {
		None => {
			Diagnostic::spanned(
				input.sig.ident.span(),
				proc_macro_error::Level::Error,
				"Expected function name".to_string(),
			)
			.emit();
			return quote! { #input }.into();
		}
		Some(c) => quote::format_ident!(
			"{}{}",
			c.to_uppercase().collect::<String>(),
			name_chars.as_str()
		),
	};

	let aliases = args.aliases.unwrap_or(vec![]);

	let function_name = input.sig.ident.clone();
	let function_generics = input.sig.generics.clone();
	let function_parameters = input.sig.inputs.clone();
	let as_params = transform_params(input.sig.inputs.clone());

	#[rustfmt::skip]
	let output = quote! {
    use async_trait::async_trait;
    use crate::util::traits::Reportable;
    use crate::commands::commander::Command;

		pub struct #type_name;

    impl #type_name {
      #input
    }

		#[async_trait]
		impl Command for #type_name {
			fn name() -> &'static str {
				#function_name_str
			}

			fn aliases() -> &'static [&'static str] {
				&[#(#aliases),*]
			}

      #[allow(unused_mut)]
			async fn execute #function_generics (#function_parameters) -> Result<(), Box<dyn Reportable>> {
				let res: Result<(), Box<dyn Reportable>> = self.#function_name #as_params 
          .await.map_err(|e| Box::new(e) as Box<dyn Reportable>);
        res
      }
		}
	};

	output.into()
}
