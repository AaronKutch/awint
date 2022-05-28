use std::collections::{HashMap, HashSet};

use awint_ext::ExtAwi;
use triple_arena::{ptr_trait_struct_with_gen, Ptr};

use crate::{bimap::BiMap, *};

ptr_trait_struct_with_gen!(Lit; Bind; Val; LtCheck; PWidth; Ref);

/// Helper struct for lowering
#[derive(Debug)]
pub(crate) struct Lower {
    // It has to be `ExtAwi` and strings all the way if we want to catch all duplication cases in
    // the tree of assignments. There might be a way to use more `Ptr`s and speed parts up.
    /// Precalculated literal values
    pub literals: BiMap<Lit, ExtAwi>,
    /// Initial `let` bindings to avoid borrowing and recalculation issues
    pub bindings: BiMap<Bind, String>,
    /// `usize` values
    pub values: BiMap<Val, String>,
    /// Pairs of values for less-than checking
    pub lt_checks: BiMap<LtCheck, (Ptr<Val>, Ptr<Val>)>,
    /// `usize` values calculated after the checks
    pub widths: BiMap<PWidth, Width>,
    /// Common bitwidth used by all concatenations
    pub common_bw: String,
    /// For all `Bits` references
    pub refs: BiMap<Ref, String>,
    pub concat_lt_partials: Vec<String>,
    pub concat_ne_partials: Vec<String>,
    pub comp_check_partials: Vec<String>,

    // used immutable and mutable refs need to be tracked separately
    /// for immutable refs
    pub used_ref_refs: HashSet<String>,
    /// for mutable refs
    pub used_mut_refs: HashSet<String>,

    /// Final literal string that we can create now
    pub s_literals: String,
    /// Final bitwidth string that we can create now
    pub s_bitwidths: String,

    // In the first refactoring we tried to remove these back references, but realized that it
    // would require a vector of backreferences in the `BiMap`. TODO in the future we should
    // refactor to use some kind of lazy tree structure that only generates strings when we
    // know they will be used, the current setup is easier to read but performance might be worse
    // than the last
    pub string_to_value: HashMap<String, String>,
    pub string_to_value_ptr: HashMap<String, Ptr<Val>>,
    pub value_to_binding: HashMap<Ptr<Val>, Ptr<Bind>>,
    pub width_to_value: HashMap<Ptr<PWidth>, Ptr<Val>>,
    pub ref_to_binding: HashMap<Ptr<Ref>, Ptr<Bind>>,
}

impl Lower {
    pub fn new(
        concats: &[Concatenation],
        dynamic_width_i: Option<usize>,
        total_bw: Option<NonZeroUsize>,
    ) -> Self {
        // create constants
        let mut literals = BiMap::<Lit, ExtAwi>::new();
        for comp in &concats[0].concatenation {
            if let Literal(ref lit) = comp.component_type {
                literals.insert(lit.clone());
            }
        }
        let mut s_literals = String::new();
        for lit in literals.arena().vals() {
            s_literals += &format!(
                "let {}_{} = {};\n",
                CONSTANT,
                lit.0,
                unstable_native_inlawi(&lit.1),
            );
        }

        // track all the bindings we will need
        let mut bindings = BiMap::<Bind, String>::new();
        // because optimizations in the binding lowering eliminates 0
        let zero_bind = bindings.insert("0".to_owned());
        for concat in concats {
            for comp in &concat.concatenation {
                lower_bindings(&mut bindings, &literals, comp);
            }
        }

        // Given a plain representation of what a component wants, this gives the
        // corresponding value
        let mut string_to_value = HashMap::<String, String>::new();
        // yes I know this is the ugliest thing ever, we need a better refactor, this is
        // needed currenly because I am not properly differentiating between logical
        // names and codegen
        let mut string_to_value_ptr = HashMap::<String, Ptr<Val>>::new();
        // we need this when determining if a binding is used.
        // note if refactoring in future that there can be 2 vals per bind
        let mut value_to_binding = HashMap::<Ptr<Val>, Ptr<Bind>>::new();

        // track all the other values we will need
        let mut values = BiMap::<Val, String>::new();
        let (p, _) = values.insert_get_ptr_and_id(format!("{}_0", BINDING));
        string_to_value.insert("0".to_owned(), format!("{}_0", VALUE));
        string_to_value_ptr.insert("0".to_owned(), p);
        value_to_binding.insert(p, zero_bind);
        // Add on bitwidth calls.
        // Note: it is important to do this before the forward bindings, otherwise
        // borrow order problems can occur.
        for concat in concats {
            for comp in &concat.concatenation {
                if let Some(name) = lowered_name(Some(&literals), comp) {
                    let (bind_id, bind_p, _) = bindings.get(&name);
                    let (val_p, val_id) =
                        values.insert_get_ptr_and_id(format!("{}_{}.bw()", BINDING, bind_id));
                    let string = name + ".bw()";
                    let value = format!("{}_{}", VALUE, val_id);
                    string_to_value.insert(string.clone(), value);
                    string_to_value_ptr.insert(string, val_p);
                    value_to_binding.insert(val_p, bind_p);
                }
            }
        }
        // forward bindings
        for (bind_p, (bind_id, s, _)) in bindings.arena() {
            let (val_p, val_id) = values.insert_get_ptr_and_id(format!("{}_{}", BINDING, bind_id));
            let value = format!("{}_{}", VALUE, val_id);
            string_to_value.insert(s.clone(), value);
            string_to_value_ptr.insert(s.clone(), val_p);
            value_to_binding.insert(val_p, bind_p);
        }

        // Create components bounds checks
        let mut lt_checks = BiMap::<LtCheck, (Ptr<Val>, Ptr<Val>)>::new();
        for concat in concats {
            for comp in &concat.concatenation {
                lower_component_checks(&mut lt_checks, &literals, &string_to_value_ptr, comp);
            }
        }
        let mut comp_check_partials: Vec<String> = Vec::new();
        for check in lt_checks.arena().vals() {
            let id0 = values.ptr_get_and_set_used(check.1 .0);
            let id1 = values.ptr_get_and_set_used(check.1 .1);
            // push a less-than check
            comp_check_partials.push(format!("({}_{}, {}_{})", VALUE, id0, VALUE, id1));
        }

        let mut width_to_value = HashMap::new();

        // track widths which will be used for concat checks and fielding
        let mut widths = BiMap::<PWidth, Width>::new();
        for concat in concats {
            for comp in &concat.concatenation {
                if let Some(width) = lower_width(Some(&literals), comp) {
                    let width_ptr = widths.insert(width.clone());
                    match width {
                        Width::Single(s) => {
                            width_to_value.insert(width_ptr, string_to_value_ptr[&s]);
                        }
                        Width::Range(s, e) => {
                            width_to_value.insert(width_ptr, string_to_value_ptr[&s]);
                            width_to_value.insert(width_ptr, string_to_value_ptr[&e]);
                        }
                    }
                }
            }
        }

        // create concatenation bounds checks
        let mut concat_lt_partials = Vec::new();
        let mut concat_ne_partials = Vec::new();
        let mut s_bitwidths = String::new();
        let mut bitwidth_partials = Vec::new();
        for (id, concat) in concats.iter().enumerate() {
            let mut partials: Vec<String> = Vec::new();
            let mut unbounded = false;
            for comp in &concat.concatenation {
                if let Some(width) = lower_width(Some(&literals), comp) {
                    partials.push(format!("{}_{}", WIDTH, widths.get_and_set_used(&width).0));
                } else {
                    unbounded = true;
                }
            }
            if partials.is_empty() {
                continue
            }
            let name = format!("{}_{}", BW, id);
            s_bitwidths += &format!("let {}: usize = {};\n", name, add_partials(partials));
            if unbounded {
                // check that we aren't trying to squeeze the unbounded filler into negative
                // widths
                if dynamic_width_i.is_none() {
                    // there should be no concat checks, and we need these for the common bitwidth
                    // calculation
                    bitwidth_partials.push(name);
                } else {
                    concat_lt_partials.push(name);
                }
            } else if dynamic_width_i.unwrap() != id {
                concat_ne_partials.push(name);
            } // else the check is redundant, if there are `n` bitwidths we only
              // need `n - 1` checks
        }

        // create the common bitwidth
        let common_bw = if let Some(bw) = total_bw {
            // note: need `: usize` because of this case
            format!("let {}: usize = {};\n", BW, bw)
        } else if dynamic_width_i.is_none() && !bitwidth_partials.is_empty() {
            // for the case with all unbounded fillers, find the max bitwidth for the buffer
            // to use.
            format!(
                "let {}: usize = Bits::unstable_max({});\n",
                BW,
                array_partials(bitwidth_partials)
            )
        } else if let Some(id) = dynamic_width_i {
            // for dynamic bitwidths, we recorded the index of one concatenation
            // which we know has a runtime deterministic bitwidth.
            let name = format!("{}_{}", BW, id);
            let s = format!("let {}: usize = {};\n", BW, name);
            s
        } else {
            String::new()
        };

        let mut ref_to_binding = HashMap::new();

        // create all references we may need
        let mut refs = BiMap::<Ref, String>::new();
        for concat in concats {
            for comp in &concat.concatenation {
                if let Some(name) = lowered_name(Some(&literals), comp) {
                    let ref_ptr = refs.insert(name.clone());
                    ref_to_binding.insert(ref_ptr, bindings.get(name).1);
                }
            }
        }

        Self {
            literals,
            bindings,
            values,
            lt_checks,
            widths,
            common_bw,
            refs,
            concat_lt_partials,
            concat_ne_partials,
            comp_check_partials,
            used_ref_refs: HashSet::new(),
            used_mut_refs: HashSet::new(),
            s_literals,
            s_bitwidths,
            string_to_value,
            string_to_value_ptr,
            value_to_binding,
            width_to_value,
            ref_to_binding,
        }
    }
}
