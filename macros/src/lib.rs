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

	let function_name = input.sig.ident.to_string();

	let mut name_chars = function_name.chars();
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

	let function_generics = input.sig.generics;
	let function_parameters = input.sig.inputs;
	let function_return = input.sig.output;
	let function_body = input.block;

	#[rustfmt::skip]
	let output = quote! {
		pub struct #type_name;

		#[async_trait]
		impl Command for #type_name {
			fn name() -> &'static str {
				#function_name
			}

			fn aliases() -> &'static [&'static str] {
				&[#(#aliases),*]
			}

			async fn execute #function_generics (#function_parameters) #function_return 
				#function_body
		}
	};

	output.into()
}
