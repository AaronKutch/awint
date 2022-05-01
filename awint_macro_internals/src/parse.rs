use proc_macro2::TokenStream;
use triple_arena::{ptr_trait_struct_with_gen, Arena, Ptr, PtrTrait};

ptr_trait_struct_with_gen!(P0);

pub struct ParseNode {
    pub s: Vec<char>,
}
