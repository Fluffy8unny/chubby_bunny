use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, AngleBracketedGenericArguments, GenericArgument, ItemStruct, PathArguments,
    Type, TypePath,
};

#[proc_macro_attribute]
pub fn chubby_bunny_bindgen(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let struct_ident = input.ident.clone();

    let game_impl_ty = match extract_game_impl_type(&input) {
        Ok(ty) => ty,
        Err(msg) => {
            return quote! {
                #input
                compile_error!(#msg);
            }
            .into();
        }
    };

    quote! {
        #[::wasm_bindgen::prelude::wasm_bindgen]
        #input

        #[::wasm_bindgen::prelude::wasm_bindgen]
        impl #struct_ident {
            #[::wasm_bindgen::prelude::wasm_bindgen(constructor)]
            pub fn new() -> Self {
                Self(::chubby_bunny_canvas_renderer::game_loop::GameLoop {
                    game_impl: ::std::boxed::Box::new(<#game_impl_ty>::new()),
                    polygon_arrays: ::std::vec::Vec::new(),
                    user_input: ::chubby_bunny_canvas_renderer::input::InputState::new(),
                })
            }

            pub fn init(&mut self, width: usize, height: usize) {
                self.0.init(width, height);
            }

            pub fn update(
                &mut self,
                dt_ms: f32,
            ) -> Result<::wasm_bindgen::JsValue, ::wasm_bindgen::JsValue> {
                self.0.update(dt_ms)
            }

            pub fn reset(&mut self, width: f32, height: f32) {
                self.0.reset(width, height);
            }

            pub fn get_polygon_arrays(
                &self,
            ) -> Result<::wasm_bindgen::JsValue, ::wasm_bindgen::JsValue> {
                self.0.get_polygon_arrays()
            }

            pub fn mouse_down(
                &mut self,
                x: f32,
                y: f32,
                mouse_button: ::chubby_bunny_canvas_renderer::input::MouseButton,
                time_stamp: f32,
            ) {
                self.0.mouse_down(x, y, mouse_button, time_stamp);
            }

            pub fn mouse_up(
                &mut self,
                x: f32,
                y: f32,
                mouse_button: ::chubby_bunny_canvas_renderer::input::MouseButton,
                time_stamp: f32,
            ) {
                self.0.mouse_up(x, y, mouse_button, time_stamp);
            }

            pub fn mouse_move(&mut self, x: f32, y: f32, time_stamp: f32) {
                self.0.mouse_move(x, y, time_stamp);
            }
        }
    }
    .into()
}

fn extract_game_impl_type(input: &ItemStruct) -> Result<Type, &'static str> {
    let field = input
        .fields
        .iter()
        .next()
        .ok_or("expected one tuple field with type GameLoop<...>")?;

    let Type::Path(TypePath { path, .. }) = &field.ty else {
        return Err("expected tuple field type to be GameLoop<...>");
    };

    let segment = path
        .segments
        .last()
        .ok_or("expected tuple field type to be GameLoop<...>")?;

    if segment.ident != "GameLoop" {
        return Err("expected tuple field type to be GameLoop<...>");
    }

    let PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) =
        &segment.arguments
    else {
        return Err("expected GameLoop to have one generic type argument");
    };

    let Some(GenericArgument::Type(game_ty)) = args.iter().next() else {
        return Err("expected GameLoop to have one generic type argument");
    };

    Ok(game_ty.clone())
}
