//! A declared type, and whether a value is written as it.

use ltk_hash::{BinHash, HashWidth};
use ltk_meta::PropertyValueEnum;
use ltk_meta::property::Kind;

use super::Shape;
use crate::bin_document::PropertyKind;
use crate::bin_walk::Declared;

/// A declared type, as one row of the table writes it.
///
/// Flat rather than recursive, because the file is: a container names its item
/// type as a bare word and carries the class beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeSpec {
    /// The type itself, such as `Map` or `String`.
    pub kind: Kind,
    /// A `Map`'s key type.
    pub key: Option<Kind>,
    /// A container's item type.
    pub value: Option<Kind>,
    /// An `Embed` or `Pointer`'s class, or the class of a container's items.
    pub class: Option<BinHash>,
    /// The item count a schema fixes a `List` at.
    pub size: Option<u32>,
    /// The byte width of a `Hash`, where a row names one. `None` matches either width.
    pub width: Option<HashWidth>,
}

impl TypeSpec {
    /// The type naming nothing but a kind, which is all the schema gives.
    #[must_use]
    pub const fn bare(kind: Kind) -> Self {
        Self {
            kind,
            key: None,
            value: None,
            class: None,
            size: None,
            width: None,
        }
    }

    /// The type `value` is written as, which is the `from` side of a
    /// schema-derived row.
    ///
    /// A `Hash` names its width, which only the value carries.
    ///
    /// # Errors
    ///
    /// A requested header that does not decode, or a `Hash` leaf that does not read.
    pub fn of<'a>(value: impl Declared<'a>) -> Result<Self, ltk_meta::Error> {
        let mut spec = Self::bare(value.kind());
        spec.key = value.key_kind()?;
        spec.value = value.item_kind()?;
        if spec.kind == Kind::Hash {
            spec.width = hash_width(value)?;
        }
        Ok(spec)
    }

    /// Whether `value` is declared as this type, over either tree.
    ///
    /// Detection reads the value's own kind rather than a version, so a table
    /// whose `from` no longer matches contributes nothing and costs one lookup.
    ///
    /// # Errors
    ///
    /// Over a view, a header that does not decode. The owned tree never fails.
    pub fn matches<'a>(&self, value: impl Declared<'a>) -> Result<bool, ltk_meta::Error> {
        if value.kind() != self.kind {
            return Ok(false);
        }
        Ok(match value.kind() {
            Kind::Container | Kind::UnorderedContainer => self.matches_items(value)?,
            Kind::Optional => self.value.is_none() || value.item_kind()? == self.value,
            Kind::Map => {
                (self.key.is_none() || value.key_kind()? == self.key)
                    && (self.value.is_none() || value.item_kind()? == self.value)
            }
            Kind::Struct | Kind::Embedded => value
                .class_hash()?
                .is_some_and(|class| self.matches_class(class)),
            Kind::Hash => match self.width {
                Some(width) => hash_width(value)? == Some(width),
                None => true,
            },
            _ => true,
        })
    }

    /// Whether a container holds the item type, and the class, this names.
    ///
    /// An empty container matches, because a container holding nothing holds
    /// nothing of the wrong class.
    fn matches_items<'a>(&self, container: impl Declared<'a>) -> Result<bool, ltk_meta::Error> {
        if self.value.is_some() && container.item_kind()? != self.value {
            return Ok(false);
        }
        let Some(class) = self.class else {
            return Ok(true);
        };
        if !matches!(container.item_kind()?, Some(Kind::Struct | Kind::Embedded)) {
            return Ok(false);
        }
        for held in container.children()? {
            let (_, item) = held?;
            if item.class_hash()? != Some(class) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn matches_class(&self, class: BinHash) -> bool {
        self.class.is_none_or(|named| named == class)
    }

    /// The type as a row draws it, in the words a bin row's tag uses: `list2[string]`
    /// or `map[hash,string]`.
    #[must_use]
    pub fn label(&self) -> String {
        let kind = tag(self.kind);
        match (self.key, self.value, self.width) {
            (Some(key), Some(value), _) => format!("{kind}[{},{}]", tag(key), tag(value)),
            (None, Some(value), _) => format!("{kind}[{}]", tag(value)),
            (_, _, Some(HashWidth::W8)) => format!("{kind}[u64]"),
            _ => kind.to_owned(),
        }
    }
}

/// The width a `Hash` leaf is stored at, read out of the value. `None` for any other kind.
///
/// # Errors
///
/// Over a view, a leaf that does not read.
fn hash_width<'a>(value: impl Declared<'a>) -> Result<Option<HashWidth>, ltk_meta::Error> {
    Ok(match value.to_value()? {
        PropertyValueEnum::Hash(hash) => Some(hash.value.width()),
        _ => None,
    })
}

impl From<Shape> for TypeSpec {
    /// The type the schema holds, which names no class and no size.
    fn from(shape: Shape) -> Self {
        Self {
            kind: shape.kind,
            key: shape.key,
            value: shape.value,
            class: None,
            size: None,
            width: None,
        }
    }
}

/// The word a bin row's tag draws for a kind, which a finding shares.
fn tag(kind: Kind) -> &'static str {
    PropertyKind::from(kind).tag()
}

#[cfg(test)]
mod tests;
