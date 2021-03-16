use cranelift_entity::{EntityRef, PrimaryMap};

use crate::{
    offset_of,
    ser::Serializer,
    std_impl::chd::{ArchivedHashMap, ArchivedHashMapResolver},
    std_impl::{ArchivedVec, VecResolver},
    Archive, Archived, ArchivedUsize, Deserialize, DeserializeUnsized, Fallible, MetadataResolver,
    RawRelPtr, Serialize,
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

// work around cranelift-entity doesn't expose elems
pub struct PrimaryMapPub<K, V>
where
    K: EntityRef,
{
    pub elems: Vec<V>,
    pub unused: PhantomData<K>,
}

pub struct ArchivedPrimaryMap<K: EntityRef, V>(ArchivedVec<V>, PhantomData<K>);

impl<K: Archive + EntityRef, V: Archive> Archive for PrimaryMap<K, V>
where
    K::Archived: EntityRef,
{
    type Archived = ArchivedPrimaryMap<K::Archived, V::Archived>;
    type Resolver = VecResolver<MetadataResolver<[V]>>;

    fn resolve(&self, pos: usize, resolver: Self::Resolver) -> Self::Archived {
        #[allow(clippy::unit_arg)]
        unsafe {
            ArchivedPrimaryMap(
                Vec::resolve(
                    &core::mem::transmute::<&PrimaryMap<K, V>, &PrimaryMapPub<K, V>>(self).elems,
                    pos,
                    resolver,
                ),
                PhantomData,
            )
        }
    }
}

impl<K: Serialize<S> + EntityRef, V: Serialize<S>, S: Serializer + ?Sized> Serialize<S>
    for PrimaryMap<K, V>
where
    K::Archived: Hash + EntityRef,
{
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        unsafe { core::mem::transmute::<&PrimaryMap<K, V>, &PrimaryMapPub<K, V>>(self) }
            .elems
            .serialize(serializer)
    }
}

impl<K: Archive + EntityRef, V: Archive, D: Fallible + ?Sized> Deserialize<PrimaryMap<K, V>, D>
    for Archived<PrimaryMap<K, V>>
where
    K::Archived: Deserialize<K, D> + EntityRef,
    V::Archived: Deserialize<V, D>,
    [V::Archived]: DeserializeUnsized<[V], D>,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<PrimaryMap<K, V>, D::Error> {
        let elems: Vec<_> = self.0.deserialize(deserializer)?;
        let result = PrimaryMapPub {
            elems,
            unused: PhantomData,
        };
        Ok(unsafe { core::mem::transmute::<PrimaryMapPub<K, V>, PrimaryMap<K, V>>(result) })
    }
}
