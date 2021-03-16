use indexmap::IndexMap;

use crate::{
    offset_of,
    ser::Serializer,
    std_impl::chd::{ArchivedHashMap, ArchivedHashMapResolver},
    Archive, Archived, ArchivedUsize, Deserialize, Fallible, RawRelPtr, Serialize,
};

use core::{
    borrow::Borrow,
    cmp::Reverse,
    hash::{Hash, Hasher},
    iter::FusedIterator,
    marker::PhantomData,
    mem::size_of,
    ops::Index,
    pin::Pin,
    slice,
};

impl<K: Archive + Hash + Eq, V: Archive> Archive for IndexMap<K, V>
where
    K::Archived: Hash + Eq,
{
    type Archived = ArchivedHashMap<K::Archived, V::Archived>;
    type Resolver = ArchivedHashMapResolver;

    fn resolve(&self, pos: usize, resolver: Self::Resolver) -> Self::Archived {
        resolver.resolve_from_len(pos, self.len())
    }
}

impl<K: Serialize<S> + Hash + Eq, V: Serialize<S>, S: Serializer + ?Sized> Serialize<S>
    for IndexMap<K, V>
where
    K::Archived: Hash + Eq,
{
    // TODO: this is incorrect, lose indexmap's order. correct impl need more work: impl an ArchivedIndexMap
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(ArchivedHashMap::serialize_from_iter(
            self.iter(),
            self.len(),
            serializer,
        )?)
    }
}

impl<K: Archive + Hash + Eq, V: Archive, D: Fallible + ?Sized> Deserialize<IndexMap<K, V>, D>
    for Archived<IndexMap<K, V>>
where
    K::Archived: Deserialize<K, D> + Hash + Eq,
    V::Archived: Deserialize<V, D>,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<IndexMap<K, V>, D::Error> {
        let mut result = IndexMap::new();
        for (k, v) in self.iter() {
            result.insert(k.deserialize(deserializer)?, v.deserialize(deserializer)?);
        }
        Ok(result)
    }
}
