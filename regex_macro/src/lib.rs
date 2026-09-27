use proc_macro2::TokenStream;
use rootcause::report;
use unsynn::ToTokens;
use unsynn::quote;
use unsynn::{LiteralString, Parse, TokenIter, unsynn};

unsynn! {
    struct Input {
        value: String,
    }
}

#[proc_macro]
pub fn checked_regex(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match parse(TokenStream::from(input)) {
        Ok(ts) => ts.into(),
        Err(err) => {
            let err_str = LiteralString::from_str(format!("{}", err));
            quote! {
                compile_error!(#err_str)
            }
            .into()
        }
    }
}

fn parse(ts: TokenStream) -> rootcause::Result<TokenStream> {
    let mut i = TokenIter::new(ts);
    let data = String::parse(&mut i).map_err(|e| report!("{}", e))?;

    regex::Regex::new(&data)?;

    Ok(quote! {
        regex::regex!(#data)
    })
}
