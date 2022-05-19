use triple_arena::{ptr_trait_struct_with_gen, Arena, Ptr};

// Previous implementation attempts all resulted in having to parse the same
// things multiple times. We must use a custom tree, and different structs can
// point at which part of the tree they correspond to. This also improves
// errors.

ptr_trait_struct_with_gen!(PText; PComp);

#[derive(Debug, Clone, Copy)]
pub enum Delimiter {
    None,
    Space,
    Parenthesis,
    Bracket,
    Brace,
    RangeBracket,
    Component,
    Concatenation,
}

impl Delimiter {
    pub fn lhs_text(&self) -> &'static str {
        use Delimiter::*;
        match self {
            None => "",
            Space => " ",
            Parenthesis => "(",
            Bracket => "[",
            Brace => "{",
            RangeBracket => "[",
            Component => "",
            Concatenation => "",
        }
    }

    pub fn rhs_text(&self) -> &'static str {
        use Delimiter::*;
        match self {
            None => "",
            Space => " ",
            Parenthesis => ")",
            Bracket => "]",
            Brace => "}",
            RangeBracket => "]",
            Component => ",",
            Concatenation => ";",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Text {
    Chars(Vec<char>),
    Group(Delimiter, Ptr<PText>),
}

#[derive(Debug)]
pub struct Ast {
    pub text: Arena<PText, Vec<Text>>,
    pub comps: Arena<PComp, Ptr<PText>>,
}

#[cfg(feature = "dbg")]
mod dbg {
    use triple_arena_render::{DebugNode, DebugNodeTrait};

    use super::*;
    use crate::chars_to_string;
    impl DebugNodeTrait<PText> for Vec<Text> {
        fn debug_node(this: &Self) -> DebugNode<PText> {
            let mut node = DebugNode {
                sources: vec![],
                center: vec![],
                sinks: vec![],
            };
            for text in this {
                match text {
                    Text::Chars(s) => {
                        if node.center.is_empty() {
                            node.center.push(chars_to_string(s));
                        } else {
                            node.center[0] += &chars_to_string(s);
                        }
                    }
                    Text::Group(d, p) => {
                        node.sources.push((*p, String::new()));
                        if node.center.is_empty() {
                            node.center.push(d.lhs_text().to_owned() + d.rhs_text());
                        } else {
                            node.center[0] += &d.lhs_text();
                            node.center[0] += &d.rhs_text();
                        }
                    }
                }
            }
            node
        }
    }
}
