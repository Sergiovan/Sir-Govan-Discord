extern crate proc_macro;

use quote::{quote, quote_spanned};
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

#[proc_macro_attribute]
pub fn command(
	args: proc_macro::TokenStream,
	item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let args = syn::parse_macro_input!(args as CommandArguments);
	let input = syn::parse_macro_input!(item as syn::ItemFn);
	let function_name = input.sig.ident;

	let function_name_str = function_name.to_string();
	let mut name_chars = function_name_str.chars();
	let capitalized_name = match name_chars.next() {
		None => {
			return quote_spanned! {
				function_name.span() => compile_error!("Expected function name");
			}
			.into()
		}
		Some(c) => c.to_uppercase().collect::<String>() + name_chars.as_str(),
	};

	let type_name = syn::Ident::new(&capitalized_name, function_name.span());

	let stringified = syn::LitStr::new(&function_name.to_string(), function_name.span());
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
				#stringified
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
