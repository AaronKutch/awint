//! Code responsible for lowering fielding ops

use crate::*;

impl Lower {
    fn field_to_awi(&mut self, comp: &Component, width: Width, other_align: bool) -> String {
        let mut field = String::new();
        let s_width = format!("{}_{}", WIDTH, self.widths.get_and_set_used(&width).0);
        if other_align {
            // subtract the `SHL` amount first
            field += &format!("{} -= {};", SHL, s_width);
        }
        if let Some(comp_name) = lowered_name(Some(&self.literals), comp) {
            let start_val = comp.range.start.clone().unwrap().code_gen_value();
            // fields `s_width` bits of from `start_val` in `comp_name` to `SHL` in
            // `AWI_REF`.
            field += &format!(
                "{}.field({}, {}_{}, {}, {}).unwrap();",
                AWI_REF,
                SHL,
                REF,
                self.refs.get_id(&comp_name),
                self.string_to_value[&start_val],
                s_width,
            );
            self.values.set_used(self.string_to_value_ptr[&start_val]);
            self.used_ref_refs.insert(comp_name);
        } // else is a filler
          // this runs for both fillers and nonfillers
        if other_align {
            field += "\n";
        } else {
            // add to the `SHL` afterwards
            field += &format!("{} += {};\n", SHL, s_width);
        }
        field
    }

    pub(crate) fn lower_fielding_to_awi(&mut self, concat: &Concatenation) -> String {
        let mut fielding = String::new();
        // Construct the value of `AWI`.
        fielding += &format!("let mut {}: usize = 0;\n", SHL);
        let mut lsb_i = 0;
        while lsb_i < concat.concatenation.len() {
            let comp = &concat.concatenation[lsb_i];
            if let Some(width) = lower_width(Some(&self.literals), comp) {
                fielding += &self.field_to_awi(comp, width, false);
            } else {
                // if we encounter an unbounded filler, reset and try again from the most
                // significant side
                break
            };
            lsb_i += 1;
        }
        let mut msb_i = concat.concatenation.len() - 1;
        if msb_i > lsb_i {
            fielding += &format!("let mut {}: usize = {};\n", SHL, BW);
        }
        while msb_i > lsb_i {
            let comp = &concat.concatenation[msb_i];
            let width = lower_width(Some(&self.literals), comp).unwrap();
            fielding += &self.field_to_awi(comp, width, true);
            msb_i -= 1;
        }
        fielding
    }

    fn field_from_awi(&mut self, comp: &Component, width: Width, other_align: bool) -> String {
        let mut field = String::new();
        let s_width = format!("{}_{}", WIDTH, self.widths.get_id(&width));
        if other_align {
            // subtract the `SHL` amount first
            field += &format!("{} -= {};", SHL, s_width);
        }
        if let Some(comp_name) = lowered_name(None, comp) {
            let start_val = comp.range.start.clone().unwrap().code_gen_value();
            // fields `s_width` bits of from `start_val` in `AWI_REF` to `SHL` in
            // `comp_name`.
            field += &format!(
                "{}_{}.field({}, {}, {}, {}).unwrap();",
                REF,
                self.refs.get_id(&comp_name),
                self.string_to_value[&start_val],
                AWI_REF,
                SHL,
                s_width,
            );
            self.values.set_used(self.string_to_value_ptr[&start_val]);
            self.used_mut_refs.insert(comp_name);
        } // else is a filler
          // this runs for both fillers and nonfillers
        if other_align {
            field += "\n";
        } else {
            // add to the `SHL` afterwards
            field += &format!("{} += {};\n", SHL, s_width);
        }
        field
    }

    // Assumes that `AWI` has been constructed and can be used as the source.
    pub(crate) fn lower_fielding_from_awi(&mut self, concat: &Concatenation) -> String {
        let mut fielding = String::new();
        fielding += &format!("let mut {}: usize = 0;\n", SHL);
        let mut lsb_i = 0;
        while lsb_i < concat.concatenation.len() {
            let comp = &concat.concatenation[lsb_i];
            // if we encounter an unbounded filler, reset and try again from the most
            // significant side
            if let Some(width) = lower_width(None, comp) {
                fielding += &self.field_from_awi(comp, width, false);
            } else {
                break
            };
            lsb_i += 1;
        }
        let mut msb_i = concat.concatenation.len() - 1;
        if msb_i > lsb_i {
            fielding += &format!("let mut {}: usize = {};\n", SHL, BW);
        }
        while msb_i > lsb_i {
            let comp = &concat.concatenation[msb_i];
            let width = lower_width(None, comp).unwrap();
            fielding += &self.field_from_awi(comp, width, true);
            msb_i -= 1;
        }
        fielding
    }
}
