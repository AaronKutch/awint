use std::ops::Range;

use triple_arena::{ptr_trait_struct_with_gen, Arena, Ptr};

use crate::concatenation::Concatenation;

// Previous implementation attempts all resulted in having to parse the same
// things multiple times. We must use a custom tree, and different structs can
// point at which part of the tree they correspond to. This also improves
// errors.

ptr_trait_struct_with_gen!(PText);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delimiter {
    None,
    Space,
    Parenthesis,
    Bracket,
    Brace,
    // this is mainly for error display purposes
    RangeBracket,
    Component,
    Concatenation,
}

impl Delimiter {
    pub const fn lhs_chars(&self) -> &'static [char] {
        use Delimiter::*;
        match self {
            None => &[],
            Space => &[' '],
            Parenthesis => &['('],
            Bracket => &['['],
            Brace => &['{'],
            RangeBracket => &['['],
            Component => &[],
            Concatenation => &[],
        }
    }

    pub const fn rhs_chars(&self) -> &'static [char] {
        use Delimiter::*;
        match self {
            None => &[],
            Space => &[' '],
            Parenthesis => &[')'],
            Bracket => &[']'],
            Brace => &['}'],
            RangeBracket => &[']'],
            Component => &[','],
            Concatenation => &[';'],
        }
    }

    pub const fn lhs_str(&self) -> &'static str {
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

    pub const fn rhs_str(&self) -> &'static str {
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
    pub txt: Arena<PText, Vec<Text>>,
    pub text_root: Ptr<PText>,
    pub cc: Vec<Concatenation>,
}

impl Ast {
    /// Converts the subtrees of `txt` into their combined `Vec<char>` form
    pub fn chars_assign_subtree(&self, chars: &mut Vec<char>, txt: Ptr<PText>) {
        let mut stack: Vec<(Ptr<PText>, usize)> = vec![(txt, 0)];
        loop {
            let last = stack.len() - 1;
            if let Some(txt) = self.txt[stack[last].0].get(stack[last].1) {
                match txt {
                    Text::Group(d, p) => {
                        chars.extend(d.lhs_chars());
                        stack.push((*p, 0));
                    }
                    Text::Chars(s) => {
                        chars.extend(s);
                        stack[last].1 += 1;
                    }
                }
            } else {
                if last == 0 {
                    break
                }
                stack.pop();
                let last = stack.len() - 1;
                if let Text::Group(d, _) = self.txt[stack[last].0][stack[last].1] {
                    chars.extend(d.rhs_chars());
                }
                stack[last].1 += 1;
            }
        }
    }

    /// Combines a range of `Text`s in `txt` into a new `Text::Chars` node
    pub fn combine_subtree(&mut self, txt: Ptr<PText>, range: Range<usize>) -> Ptr<PText> {
        let mut chars = vec![];
        for i in range {
            match &self.txt[txt][i] {
                Text::Group(d, p) => {
                    chars.extend(d.lhs_chars());
                    self.chars_assign_subtree(&mut chars, *p);
                    chars.extend(d.rhs_chars());
                }
                Text::Chars(s) => {
                    chars.extend(s);
                }
            }
        }
        self.txt.insert(vec![Text::Chars(chars)])
    }
}

#[cfg(feature = "debug")]
mod debug {
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
            for txt in this {
                match txt {
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
                            node.center.push(d.lhs_str().to_owned() + d.rhs_str());
                        } else {
                            node.center[0] += &d.lhs_str();
                            node.center[0] += &d.rhs_str();
                        }
                    }
                }
            }
            node
        }
    }
}
