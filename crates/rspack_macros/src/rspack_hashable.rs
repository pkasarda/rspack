use proc_macro2::TokenStream;
use quote::quote;
use syn::{
  Data, DeriveInput, Error, Fields, Generics, Ident, Index, Path, Result, TypeParamBound,
  parse_quote,
};

#[derive(Default)]
struct FieldOptions {
  skip: bool,
  order: Option<usize>,
}

pub fn expand_rspack_hashable_derive(input: DeriveInput) -> Result<TokenStream> {
  let hash_crate = rspack_hash_crate_path(&input)?;
  let body = match &input.data {
    Data::Struct(data) => hash_fields(&hash_crate, &data.fields)?,
    Data::Enum(data) => {
      let arms = data
        .variants
        .iter()
        .map(|variant| hash_variant(&hash_crate, &input.ident, variant))
        .collect::<Result<Vec<_>>>()?;
      quote! {
        match self {
          #(#arms)*
        }
      }
    }
    Data::Union(data) => {
      return Err(Error::new_spanned(
        data.union_token,
        "RspackHashable cannot be derived for unions",
      ));
    }
  };

  let ident = &input.ident;
  let mut generics = input.generics.clone();
  add_rspack_hashable_bounds(&mut generics, &hash_crate);
  let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

  Ok(quote! {
    impl #impl_generics #hash_crate::RspackHashable for #ident #ty_generics #where_clause {
      fn hash(&self, state: &mut #hash_crate::RspackHash) {
        #body
      }
    }
  })
}

fn rspack_hash_crate_path(input: &DeriveInput) -> Result<Path> {
  let mut hash_crate = None;
  for attr in &input.attrs {
    if !attr.path().is_ident("rspack_hash") {
      continue;
    }
    attr.parse_nested_meta(|meta| {
      if meta.path.is_ident("crate") {
        let value = meta.value()?;
        hash_crate = Some(value.parse::<Path>()?);
        Ok(())
      } else {
        Err(meta.error("unsupported rspack_hash attribute"))
      }
    })?;
  }
  Ok(hash_crate.unwrap_or_else(|| parse_quote!(::rspack_hash)))
}

fn field_options(attrs: &[syn::Attribute]) -> Result<FieldOptions> {
  let mut options = FieldOptions::default();
  for attr in attrs {
    if !attr.path().is_ident("rspack_hash") {
      continue;
    }
    attr.parse_nested_meta(|meta| {
      if meta.path.is_ident("skip") {
        options.skip = true;
        Ok(())
      } else if meta.path.is_ident("order") {
        let value = meta.value()?;
        let order = value.parse::<syn::LitInt>()?;
        options.order = Some(order.base10_parse()?);
        Ok(())
      } else {
        Err(meta.error("unsupported rspack_hash field attribute"))
      }
    })?;
  }
  Ok(options)
}

fn hash_fields(hash_crate: &Path, fields: &Fields) -> Result<TokenStream> {
  match fields {
    Fields::Named(fields) => {
      let fields = fields
        .named
        .iter()
        .enumerate()
        .filter_map(|(index, field)| {
          let options = match field_options(&field.attrs) {
            Ok(options) => options,
            Err(err) => return Some(Err(err)),
          };
          if options.skip {
            return Some(Ok(None));
          }
          let ident = field.ident.as_ref()?;
          Some(Ok(Some((
            options.order.unwrap_or(index),
            quote! {
              #hash_crate::RspackHashable::hash(&self.#ident, state);
            },
          ))))
        })
        .collect::<Result<Vec<_>>>()?;
      let mut fields = fields.into_iter().flatten().collect::<Vec<_>>();
      sort_fields(&mut fields)?;
      let fields = fields.into_iter().map(|(_, hash)| hash);
      Ok(quote! {
        #(#fields)*
      })
    }
    Fields::Unnamed(fields) => {
      let fields = fields
        .unnamed
        .iter()
        .enumerate()
        .filter_map(|(index, field)| {
          let options = match field_options(&field.attrs) {
            Ok(options) => options,
            Err(err) => return Some(Err(err)),
          };
          if options.skip {
            return Some(Ok(None));
          }
          let field_index = Index::from(index);
          Some(Ok(Some((
            options.order.unwrap_or(index),
            quote! {
              #hash_crate::RspackHashable::hash(&self.#field_index, state);
            },
          ))))
        })
        .collect::<Result<Vec<_>>>()?;
      let mut fields = fields.into_iter().flatten().collect::<Vec<_>>();
      sort_fields(&mut fields)?;
      let fields = fields.into_iter().map(|(_, hash)| hash);
      Ok(quote! {
        #(#fields)*
      })
    }
    Fields::Unit => Ok(TokenStream::new()),
  }
}

fn hash_variant(
  hash_crate: &Path,
  enum_ident: &Ident,
  variant: &syn::Variant,
) -> Result<TokenStream> {
  let variant_ident = &variant.ident;
  let variant_name = variant_ident.to_string();
  match &variant.fields {
    Fields::Named(fields) => {
      let field_idents = fields
        .named
        .iter()
        .map(|field| {
          field
            .ident
            .clone()
            .ok_or_else(|| Error::new_spanned(field, "expected named field"))
        })
        .collect::<Result<Vec<_>>>()?;
      let hashes = fields
        .named
        .iter()
        .zip(&field_idents)
        .enumerate()
        .filter_map(|(index, (field, ident))| {
          let options = match field_options(&field.attrs) {
            Ok(options) => options,
            Err(err) => return Some(Err(err)),
          };
          if options.skip {
            return Some(Ok(None));
          }
          Some(Ok(Some((
            options.order.unwrap_or(index),
            quote! {
              #hash_crate::RspackHashable::hash(#ident, state);
            },
          ))))
        })
        .collect::<Result<Vec<_>>>()?;
      let mut hashes = hashes.into_iter().flatten().collect::<Vec<_>>();
      sort_fields(&mut hashes)?;
      let hashes = hashes.into_iter().map(|(_, hash)| hash);
      Ok(quote! {
        #enum_ident::#variant_ident { #(#field_idents),* } => {
          #hash_crate::RspackHashable::hash(#variant_name, state);
          #(#hashes)*
        }
      })
    }
    Fields::Unnamed(fields) => {
      let bindings = (0..fields.unnamed.len())
        .map(|index| Ident::new(&format!("field_{index}"), variant_ident.span()))
        .collect::<Vec<_>>();
      let hashes = fields
        .unnamed
        .iter()
        .zip(&bindings)
        .enumerate()
        .filter_map(|(index, (field, ident))| {
          let options = match field_options(&field.attrs) {
            Ok(options) => options,
            Err(err) => return Some(Err(err)),
          };
          if options.skip {
            return Some(Ok(None));
          }
          Some(Ok(Some((
            options.order.unwrap_or(index),
            quote! {
              #hash_crate::RspackHashable::hash(#ident, state);
            },
          ))))
        })
        .collect::<Result<Vec<_>>>()?;
      let mut hashes = hashes.into_iter().flatten().collect::<Vec<_>>();
      sort_fields(&mut hashes)?;
      let hashes = hashes.into_iter().map(|(_, hash)| hash);
      Ok(quote! {
        #enum_ident::#variant_ident(#(#bindings),*) => {
          #hash_crate::RspackHashable::hash(#variant_name, state);
          #(#hashes)*
        }
      })
    }
    Fields::Unit => Ok(quote! {
      #enum_ident::#variant_ident => {
        #hash_crate::RspackHashable::hash(#variant_name, state);
      }
    }),
  }
}

fn sort_fields(fields: &mut [(usize, TokenStream)]) -> Result<()> {
  fields.sort_by_key(|(order, _)| *order);
  for window in fields.windows(2) {
    if window[0].0 == window[1].0 {
      return Err(Error::new_spanned(
        &window[1].1,
        "duplicate rspack_hash field order",
      ));
    }
  }
  Ok(())
}

fn add_rspack_hashable_bounds(generics: &mut Generics, hash_crate: &Path) {
  for param in generics.type_params_mut() {
    let bound: TypeParamBound = parse_quote!(#hash_crate::RspackHashable);
    param.bounds.push(bound);
  }
}
