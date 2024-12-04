use alloc::vec::Vec;
use alloy_primitives::{hex, B256};
use core::fmt;

/// Hash builder value.
///
/// Stores [`HashBuilderValueRef`] efficiently by reusing resources.
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HashBuilderValue {
    /// Stores the bytes of either the leaf node value or the hash of adjacent nodes.
    #[cfg_attr(feature = "serde", serde(with = "hex"))]
    buf: Vec<u8>,
    /// The kind of value that is stored in `buf`.
    kind: HashBuilderValueKind,
    #[cfg_attr(feature = "serde", serde(skip))]
    _hash: B256,
}

impl Default for HashBuilderValue {
    fn default() -> Self {
        Self {
            buf: Vec::with_capacity(128),
            kind: HashBuilderValueKind::default(),
            _hash: B256::default(),
        }
    }
}

impl fmt::Debug for HashBuilderValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

#[cfg(feature = "arbitrary")]
impl<'u> arbitrary::Arbitrary<'u> for HashBuilderValue {
    fn arbitrary(g: &mut arbitrary::Unstructured<'u>) -> arbitrary::Result<Self> {
        let kind = HashBuilderValueKind::arbitrary(g)?;
        let (buf, _hash) = match kind {
            HashBuilderValueKind::Bytes => (Vec::arbitrary(g)?, B256::default()),
            HashBuilderValueKind::Hash => {
                let _hash = B256::arbitrary(g)?;
                (_hash.to_vec(), _hash)
            },
        };
        Ok(Self { buf, kind, _hash })
    }
}

#[cfg(feature = "arbitrary")]
impl proptest::arbitrary::Arbitrary for HashBuilderValue {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;

        proptest::arbitrary::any::<HashBuilderValueKind>()
            .prop_flat_map(|kind| {
                let range = match kind {
                    HashBuilderValueKind::Bytes => 0..=128,
                    HashBuilderValueKind::Hash => 32..=32,
                };
                proptest::collection::vec(any::<u8>(), range)
                    .prop_map(move |buf| {
                        let _hash = if kind == HashBuilderValueKind::Hash {
                            B256::from_slice(&buf)
                        } else {
                            B256::default()
                        };
                        Self { buf, kind, _hash }
                    })
            })
            .boxed()
    }
}

impl HashBuilderValue {
    /// Creates a new empty value.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the value as a reference.
    #[inline]
    pub fn as_ref(&self) -> HashBuilderValueRef<'_> {
        match self.kind {
            HashBuilderValueKind::Bytes => HashBuilderValueRef::Bytes(&self.buf),
            HashBuilderValueKind::Hash => {
                debug_assert_eq!(self.buf.len(), 32);
                HashBuilderValueRef::Hash(&self._hash)
            }
        }
    }

    /// Returns the value as a slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.buf
    }

    /// Like `set_from_ref`, but takes ownership of the bytes.
    pub fn set_bytes_owned(&mut self, bytes: Vec<u8>) {
        self.buf = bytes;
        self.kind = HashBuilderValueKind::Bytes;
    }

    /// Sets the value from the given bytes.
    #[inline]
    pub fn set_from_ref(&mut self, value: HashBuilderValueRef<'_>) {
        self.buf.clear();
        self.buf.extend_from_slice(value.as_slice());
        self.kind = value.kind();
        self._hash = match value {
            HashBuilderValueRef::Bytes(_) => B256::default(),
            HashBuilderValueRef::Hash(hash) => *hash,
        };
    }

    /// Clears the value.
    #[inline]
    pub fn clear(&mut self) {
        self.buf.clear();
        self.kind = HashBuilderValueKind::default();
        self._hash = B256::default();
    }
}

/// The kind of the current hash builder value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(derive_arbitrary::Arbitrary, proptest_derive::Arbitrary))]
enum HashBuilderValueKind {
    /// Value of the leaf node.
    #[default]
    Bytes,
    /// Hash of adjacent nodes.
    Hash,
}

/// Hash builder value reference.
pub enum HashBuilderValueRef<'a> {
    /// Value of the leaf node.
    Bytes(&'a [u8]),
    /// Hash of adjacent nodes.
    Hash(&'a B256),
}

impl<'a> HashBuilderValueRef<'a> {
    /// Returns the value as a slice.
    pub const fn as_slice(&self) -> &'a [u8] {
        match *self {
            HashBuilderValueRef::Bytes(bytes) => bytes,
            HashBuilderValueRef::Hash(hash) => hash.as_slice(),
        }
    }

    /// Returns the kind of the value.
    const fn kind(&self) -> HashBuilderValueKind {
        match *self {
            HashBuilderValueRef::Bytes(_) => HashBuilderValueKind::Bytes,
            HashBuilderValueRef::Hash(_) => HashBuilderValueKind::Hash,
        }
    }
}

impl fmt::Debug for HashBuilderValueRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match *self {
            HashBuilderValueRef::Bytes(_) => "Bytes",
            HashBuilderValueRef::Hash(_) => "Hash",
        };
        let slice = hex::encode_prefixed(self.as_slice());
        write!(f, "{name}({slice})")
    }
}
