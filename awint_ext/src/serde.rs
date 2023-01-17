use core::fmt;

use awint_core::bw;
use serde::{
    de,
    de::{MapAccess, SeqAccess, Visitor},
    ser::{SerializeStruct, SerializeTuple},
    Deserialize, Deserializer, Serialize, Serializer,
};

use crate::ExtAwi;

/// A `serde_support` impl
impl Serialize for ExtAwi {
    /// Serializes `self` in a platform independent way. In human readable form,
    /// it serializes into a struct named "ExtAwi" with two fields "bw" and
    /// "bits". "bw" is the bitwidth in decimal, and "bits" are an unsigned
    /// hexadecimal string equivalent to what would be generated from
    /// `ExtAwi::bits_to_string_radix(self.const_as_ref(), false, 16, false, 0)`
    ///
    /// ```
    /// // Example using the `ron` crate. Note that it
    /// // omits the struct name which would be "ExtAwi".
    /// use awint::{inlawi, Bits, ExtAwi, InlAwi};
    /// use ron::to_string;
    ///
    /// let awi = ExtAwi::from(inlawi!(0xfedcba9876543210u100));
    /// assert_eq!(
    ///     to_string(&awi).unwrap(),
    ///     "(bw:100,bits:\"fedcba9876543210\")"
    /// );
    /// ```
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let str_buf: &str =
            &ExtAwi::bits_to_string_radix(self.const_as_ref(), false, 16, false, 0).unwrap();
        if serializer.is_human_readable() {
            let mut s = serializer.serialize_struct("ExtAwi", 2)?;
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

struct ExtAwiVisitor;

impl<'de> Visitor<'de> for ExtAwiVisitor {
    type Value = ExtAwi;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(
            "struct ExtAwi consisting of a decimal bitwidth \"bw\" and a hexadecimal unsigned \
             integer \"bits\"",
        )
    }

    fn visit_map<V>(self, mut map: V) -> Result<ExtAwi, V::Error>
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
            return Err(de::Error::custom("`bw` field should be nonzero"))
        }
        let w = bw(w);
        let mut awi = ExtAwi::zero(w);
        let mut pad = ExtAwi::zero(w);
        let result =
            awi.const_as_mut()
                .power_of_two_bytes_(None, bits.as_bytes(), 16, pad.const_as_mut());
        if let Err(e) = result {
            return Err(de::Error::custom(e))
        }
        Ok(awi)
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<ExtAwi, V::Error>
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
            return Err(de::Error::custom("`bw` field should be nonzero"))
        }
        let w = bw(w);
        let mut awi = ExtAwi::zero(w);
        let mut pad = ExtAwi::zero(w);
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
impl<'de> Deserialize<'de> for ExtAwi {
    /// Deserializes `self` in a platform independent way.
    ///
    /// ```
    /// // Example using the `ron` crate. Note that it
    /// // omits the struct name which would be "ExtAwi".
    /// use awint::{inlawi, inlawi_ty, Bits, ExtAwi, InlAwi};
    /// use ron::from_str;
    ///
    /// let awi0 = ExtAwi::from(inlawi!(0xfedcba9876543210u100));
    /// let awi1: ExtAwi = from_str("(bw:100,bits:\"fedcba9876543210\")").unwrap();
    /// assert_eq!(awi0, awi1);
    /// ```
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct("ExtAwi", FIELDS, ExtAwiVisitor)
    }
}
