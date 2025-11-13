#[cfg(feature = "schemas")]
use schemars::JsonSchema;

/// Representation of a single permission override
#[derive(Debug, Clone, Eq, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
pub struct Override {
    /// Allow bit flags
    pub allow: u64,
    /// Disallow bit flags
    pub deny: u64,
}

/// Data permissions Field - contains both allow and deny
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
pub struct DataPermissionsField {
    pub permissions: Override,
}

/// Data permissions Value - contains allow
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
pub struct DataPermissionsValue {
    pub permissions: u64,
}

/// Data permissions Poly - can contain either Value or Field
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum DataPermissionPoly {
    Value {
        /// Permission values to set for members in a `Group`
        permissions: u64,
    },
    Field {
        /// Allow / deny values to set for members in this server channel
        permissions: Override,
    },
}

/// Representation of a single permission override
/// as it appears on models and in the database
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
pub struct OverrideField {
    /// Allow bit flags
    pub a: i64,
    /// Disallow bit flags
    pub d: i64,
}

impl Override {
    /// Into allows
    pub fn allows(&self) -> u64 {
        self.allow
    }

    /// Into denies
    pub fn denies(&self) -> u64 {
        self.deny
    }
}

impl From<Override> for OverrideField {
    fn from(v: Override) -> Self {
        Self {
            a: v.allow as i64,
            d: v.deny as i64,
        }
    }
}

impl From<OverrideField> for Override {
    fn from(v: OverrideField) -> Self {
        Self {
            allow: v.a as u64,
            deny: v.d as u64,
        }
    }
}

#[cfg(feature = "bson")]
use bson::Bson;

#[cfg(feature = "bson")]
impl From<OverrideField> for Bson {
    fn from(v: OverrideField) -> Self {
        Self::Document(bson::to_document(&v).unwrap())
    }
}
