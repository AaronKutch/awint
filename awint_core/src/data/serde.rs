use core::fmt;

use awint_internals::*;
use serde::{
    de,
    de::{MapAccess, SeqAccess, Visitor},
    ser::{SerializeStruct, SerializeTuple},
    Deserialize, Deserializer, Serialize, Serializer,
};

use crate::InlAwi;

/// A `serde_support` impl
impl<const BW: usize, const LEN: usize> Serialize for InlAwi<BW, LEN> {
    /// Serializes `self` in a platform independent way. In human readable form,
    /// it serializes into a struct named "InlAwi" with two fields "bw" and
    /// "bits". "bw" is the bitwidth in decimal, and "bits" are an unsigned
    /// hexadecimal string equivalent to what would be generated from
    /// `ExtAwi::bits_to_string_radix(self.const_as_ref(), false, 16, false, 0)`
    /// from the `awint_ext` crate.
    ///
    /// Note that there is clever use of buffers to avoid allocation on
    /// `awint`'s side when serializing `InlAwi`s.
    ///
    /// ```
    /// // Example using the `ron` crate. Note that it
    /// // omits the struct name which would be "InlAwi".
    /// use awint::{inlawi, Bits, InlAwi};
    /// use ron::to_string;
    ///
    /// let awi = inlawi!(0xfedcba9876543210u100);
    /// assert_eq!(
    ///     to_string(&awi).unwrap(),
    ///     "(bw:100,bits:\"fedcba9876543210\")"
    /// );
    /// ```
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // this is all done without allocation on our side
        let bits = self.const_as_ref();
        // TODO this buffer is ~5 times larger than needed. We have a
        // `panicking_chars_upper_bound` that can be used in array lengths if the input
        // is a constant, but annoyingly we currently can't use generic parameters for
        // it.
        let mut buf = [0u8; BW];
        let mut pad = Self::zero();
        // do the minimum amount of work necessary
        let upper = chars_upper_bound(bits.sig(), 16).unwrap();
        bits.to_bytes_radix(false, &mut buf[..upper], 16, false, pad.const_as_mut())
            .unwrap();
        // find the lower bound of signficant digits
        let mut lower = 0;
        for i in 0..upper {
            if buf[i] != b'0' {
                lower = i;
                break
            }
            if (i + 1) == upper {
                // all zeros, use one zero. `chars_upper_bound` always returns at least 1, so
                // underflow is not possible.
                lower = upper - 1;
                break
            }
        }
        let str_buf = core::str::from_utf8(&buf[lower..upper]).unwrap();
        if serializer.is_human_readable() {
            let mut s = serializer.serialize_struct("InlAwi", 2)?;
            s.serialize_field("bw", &self.bw())?;
            s.serialize_field("bits", str_buf)?;
            s.end()
        } else {
            let mut s = serializer.serialize_tuple(2)?;
            s.serialize_element(&self.bw())?;
            s.serialize_element(str_buf)?;
            s.end()
        }
    }
}

const FIELDS: &[&str] = &["bw", "bits"];

/// Helper for the deserialization impl
enum Field {
    Bw,
    Bits,
}

impl<'de> Deserialize<'de> for Field {
    fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FieldVisitor;

        impl<'de> Visitor<'de> for FieldVisitor {
            type Value = Field;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("`bw` or `bits`")
            }

            fn visit_str<E>(self, value: &str) -> Result<Field, E>
            where
                E: de::Error,
            {
                match value {
                    "bw" => Ok(Field::Bw),
                    "bits" => Ok(Field::Bits),
                    _ => Err(de::Error::unknown_field(value, FIELDS)),
                }
            }
        }

        deserializer.deserialize_identifier(FieldVisitor)
    }
}

struct InlAwiVisitor<const BW: usize, const LEN: usize>;

impl<'de, const BW: usize, const LEN: usize> Visitor<'de> for InlAwiVisitor<BW, LEN> {
    type Value = InlAwi<BW, LEN>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(
            "struct InlAwi consisting of a decimal bitwidth \"bw\" and a hexadecimal unsigned \
             integer \"bits\"",
        )
    }

    fn visit_map<V>(self, mut map: V) -> Result<InlAwi<BW, LEN>, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut w: Option<usize> = None;
        let mut bits: Option<&str> = None;
        while let Some(key) = map.next_key()? {
            match key {
                Field::Bw => {
                    if w.is_some() {
                        return Err(de::Error::duplicate_field("bw"))
                    }
                    w = Some(map.next_value()?);
                }
                Field::Bits => {
                    if bits.is_some() {
                        return Err(de::Error::duplicate_field("bits"))
                    }
                    bits = Some(map.next_value()?);
                }
            }
        }
        let w = w.ok_or_else(|| de::Error::missing_field("bw"))?;
        let bits = bits.ok_or_else(|| de::Error::missing_field("bits"))?;
        if w == 0 {
            // in case someone made `BW == 0` manually
            return Err(de::Error::custom("`bw` field should be nonzero"))
        }
        if w != BW {
            return Err(de::Error::custom(
                "`bw` field does not equal `BW` of `InlAwi<BW, LEN>` type this deserialization is \
                 happening on",
            ))
        }
        let mut awi = InlAwi::<BW, LEN>::zero();
        let mut pad = InlAwi::<BW, LEN>::zero();
        let result =
            awi.const_as_mut()
                .power_of_two_bytes_(None, bits.as_bytes(), 16, pad.const_as_mut());
        if let Err(e) = result {
            return Err(de::Error::custom(e))
        }
        Ok(awi)
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<InlAwi<BW, LEN>, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let w: usize = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let bits: &str = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;
        if w == 0 {
            // in case someone made `BW == 0` manually
            return Err(de::Error::custom("`bw` field should be nonzero"))
        }
        if w != BW {
            return Err(de::Error::custom(
                "`bw` field does not equal `BW` of `InlAwi<BW, LEN>` type this deserialization is \
                 happening on",
            ))
        }
        let mut awi = InlAwi::<BW, LEN>::zero();
        let mut pad = InlAwi::<BW, LEN>::zero();
        let result =
            awi.const_as_mut()
                .power_of_two_bytes_(None, bits.as_bytes(), 16, pad.const_as_mut());
        if let Err(e) = result {
            return Err(de::Error::custom(e))
        }
        Ok(awi)
    }
}

/// A `serde_support` impl
impl<'de, const BW: usize, const LEN: usize> Deserialize<'de> for InlAwi<BW, LEN> {
    /// Deserializes `self` in a platform independent way.
    ///
    /// ```
    /// // Example using the `ron` crate. Note that it
    /// // omits the struct name which would be "InlAwi".
    /// use awint::{inlawi, inlawi_ty, Bits, InlAwi};
    /// use ron::from_str;
    ///
    /// let awi0 = inlawi!(0xfedcba9876543210u100);
    /// // note: you will probably have to specify the type with `inlawi_ty`
    /// let awi1: inlawi_ty!(100) = from_str("(bw:100,bits:\"fedcba9876543210\")").unwrap();
    /// assert_eq!(awi0, awi1);
    /// ```
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct("InlAwi", FIELDS, InlAwiVisitor)
    }
}
