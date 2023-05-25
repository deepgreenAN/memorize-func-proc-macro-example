use std::num::NonZeroUsize;

use darling::ast::NestedMeta;
use darling::FromMeta;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_macro_input, parse_quote, ItemFn};
use syn::{Block, FnArg, Ident, ReturnType};

#[derive(Debug, FromMeta)]
struct MacroMeta {
    size: Option<usize>,
}

/// 関数をメモライズする．
#[proc_macro_attribute]
pub fn memorize_func(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut ast = parse_macro_input!(item as ItemFn);
    // マクロのアトリビュート引数
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => {
            return e.into_compile_error().into();
        }
    };
    let macro_meta = match MacroMeta::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => return e.write_errors().into(),
    };

    // キャッシュのサイズをコンパイル時にチェック
    let cache_size = {
        let size = macro_meta.size.unwrap_or(1000); // デフォルトは1000
        let _ = match NonZeroUsize::try_from(size) {
            Ok(size) => size,
            Err(e) => return darling::Error::custom(e).write_errors().into(),
        };
        size
    };

    // 関数名
    let fn_name = &ast.sig.ident;

    let fn_input = &ast.sig.inputs;

    // inputのアサーション
    // 引数が全てハッシュ可能であるかどうか
    let fn_input_assertion = {
        let fn_input_assertion_iter = fn_input.iter().filter_map(|input| {
            match input {
                // selfなどの予約語
                FnArg::Receiver(_) => None,
                // 引数:型
                FnArg::Typed(pat_type) => {
                    let ty = &pat_type.ty;
                    Some(quote_spanned! {pat_type.span()=>
                        struct _AssertHashMapKey where #ty: std::hash::Hash + Eq;
                    })
                }
            }
        });
        quote!(#(#fn_input_assertion_iter)*)
    };

    let fn_output = &ast.sig.output;

    // outputのアサーション
    // 返り値がクローン可能であるかどうか
    let fn_output_assertion = {
        let ty = match fn_output {
            ReturnType::Type(_, ty) => ty,
            ReturnType::Default => panic!("Return type must not be ()."),
        };
        quote_spanned! {ty.span()=>
            struct _AssertionClone where #ty: Clone;
        }
    };

    // 関数ブロック
    let block = &ast.block;
    // 関数の引数名
    let fn_input_name_iter = fn_input.iter().filter_map(|input| {
        match input {
            // selfなどの予約語
            FnArg::Receiver(_) => None,
            // 引数:型
            FnArg::Typed(pat_type) => Some(&pat_type.pat),
        }
    });
    // 関数の引数の型を羅列したトークン
    let fn_input_ty_iter = fn_input.iter().filter_map(|input| {
        match input {
            // selfなどの予約語
            FnArg::Receiver(_) => None,
            // 引数:型
            FnArg::Typed(pat_type) => Some(&pat_type.ty),
        }
    });
    let fn_input_names = quote! {#(#fn_input_name_iter),*};

    // 関数の返り値の型
    let fn_output_ty = match fn_output {
        ReturnType::Type(_, ty) => ty,
        ReturnType::Default => panic!("Return type must not be ()."),
    };

    // グローバルなハッシュマップの名前
    let global_map_name = {
        let fn_name_string = fn_name.to_string();
        Ident::new(
            &format!("MEMORIZE_MAP_{}", fn_name_string.to_uppercase()),
            proc_macro2::Span::call_site(),
        )
    };

    // 新しい関数プロックを定義して変更
    let new_block: Block = parse_quote! {
        {
            // キャッシュの中にあったらそれを返す
            {
                if let Some(value) = #global_map_name.lock().unwrap().get(&(#fn_input_names)) {
                    return value.clone();
                }
            }

            // 元の関数を実行
            let block_fn = |#fn_input_names| #block;
            let ret = block_fn(#fn_input_names);

            // キャッシュに追加
            {
                #global_map_name.lock().unwrap().push((#fn_input_names), ret.clone());
            }

            ret
        }
    };
    *ast.block = new_block;

    // 生成したコード
    quote! {
        // アサーション
        #fn_input_assertion
        #fn_output_assertion


        // グローバルマップ
        static #global_map_name: ::memorize_func::Lazy<
            std::sync::Mutex<
                ::memorize_func::LruCache<
                    (#(#fn_input_ty_iter),*), #fn_output_ty
                >
            >
        > = ::memorize_func::Lazy::new(||
                std::sync::Mutex::new(
                    ::memorize_func::LruCache::new(#cache_size.try_into().unwrap())
                )
        );

        // 関数
        #ast
    }
    .into()
}
