//! Provides "raw perl value" support with a trick similar to how the toml crate's `Spanned` type
//! works.

use std::cell::RefCell;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::Value;

pub(crate) const NAME: &str = "$__perlmod_private_RawValue";
pub(crate) const VALUE: &str = "$__perlmod_private_raw_value";

thread_local!(static SERIALIZE_RAW: RefCell<bool> = RefCell::new(false));

pub(crate) struct RawGuard(bool);

#[inline]
pub(crate) fn guarded(on: bool) -> RawGuard {
    SERIALIZE_RAW.with(move |raw| RawGuard(raw.replace(on)))
}

#[inline]
pub(crate) fn is_enabled() -> bool {
    SERIALIZE_RAW.with(|raw| *raw.borrow())
}

/// A raw perl value. This is a type hint that this contains a raw reference and can *only* be
/// deserialized from a perlmod deserializer.
///
/// It should also not be serialized by anythin gother than `perlmod`'s serialization mechanisms.
#[derive(Clone)]
pub struct RawValue {
    value: Value,
}

impl RawValue {
    /// Consume thie `RawValue` to get the contained [`Value`].
    pub fn into_inner(self) -> Value {
        self.value
    }

    /// Get the contaiend [`Value`] by reference.
    pub fn get_ref(&self) -> &Value {
        &self.value
    }

    /// Get the contaiend [`Value`] by reference.
    pub fn get_mut(&mut self) -> &mut Value {
        &mut self.value
    }
}

impl From<Value> for RawValue {
    fn from(value: Value) -> Self {
        Self { value }
    }
}

impl std::ops::Deref for RawValue {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl std::ops::DerefMut for RawValue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<'de> Deserialize<'de> for RawValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Error, Visitor};

        struct V;

        impl<'de> Visitor<'de> for V {
            type Value = RawValue;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a raw perl value")
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<RawValue, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                if visitor.next_key()? != Some(VALUE) {
                    return Err(Error::custom("raw value key not found"));
                }

                let sv: usize = visitor.next_value()?;

                Ok(RawValue {
                    value: unsafe { Value::from_raw_ref(sv as *mut crate::ffi::SV) },
                })
            }
        }

        deserializer.deserialize_struct(NAME, &[VALUE], V)
    }
}

impl Serialize for RawValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let _guard = guarded(true);
        serialize_raw(&self.value, serializer)
    }
}

pub(crate) fn serialize_raw<S>(sv: &crate::ScalarRef, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeStruct;

    let mut s = serializer.serialize_struct(NAME, 1)?;
    s.serialize_field(VALUE, &(sv.sv() as usize))?;
    s.end()
}
