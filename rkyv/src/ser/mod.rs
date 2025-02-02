//! Serialization traits, serializers, and adapters.

#[cfg(feature = "std")]
pub mod adapters;
pub mod serializers;

use crate::{
    Archive, ArchivePointee, ArchiveUnsized, Archived, Fallible, RelPtr, Serialize,
    SerializeUnsized,
};
use core::{mem, slice};

/// A byte sink that knows where it is.
///
/// A type that is [`io::Write`](std::io::Write) can be wrapped in a
/// [`WriteSerializer`](serializers::WriteSerializer) to equip it with
/// `Serializer`.
///
/// It's important that the memory for archived objects is properly aligned
/// before attempting to read objects out of it; use the
/// [`Aligned`](crate::Aligned) wrapper if it's appropriate.
pub trait Serializer: Fallible {
    /// Returns the current position of the serializer.
    fn pos(&self) -> usize;

    /// Attempts to write the given bytes to the serializer.
    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;

    /// Advances the given number of bytes as padding.
    fn pad(&mut self, mut padding: usize) -> Result<(), Self::Error> {
        const ZEROES_LEN: usize = 16;
        const ZEROES: [u8; ZEROES_LEN] = [0; ZEROES_LEN];

        while padding > 0 {
            let len = usize::min(ZEROES_LEN, padding);
            self.write(&ZEROES[0..len])?;
            padding -= len;
        }

        Ok(())
    }

    /// Aligns the position of the serializer to the given alignment.
    fn align(&mut self, align: usize) -> Result<usize, Self::Error> {
        debug_assert!(align & (align - 1) == 0);

        let offset = self.pos() & (align - 1);
        if offset != 0 {
            self.pad(align - offset)?;
        }
        Ok(self.pos())
    }

    /// Aligns the position of the serializer to be suitable to write the given
    /// type.
    fn align_for<T>(&mut self) -> Result<usize, Self::Error> {
        self.align(mem::align_of::<T>())
    }

    /// Resolves the given value with its resolver and writes the archived type.
    ///
    /// Returns the position of the written archived type.
    ///
    /// # Safety
    ///
    /// This is only safe to call when the serializer is already aligned for the
    /// archived version of the given type.
    unsafe fn resolve_aligned<T: Archive + ?Sized>(
        &mut self,
        value: &T,
        resolver: T::Resolver,
    ) -> Result<usize, Self::Error> {
        let pos = self.pos();
        debug_assert!(pos & (mem::align_of::<T::Archived>() - 1) == 0);
        let archived = &value.resolve(pos, resolver);
        let data = (archived as *const T::Archived).cast::<u8>();
        let len = mem::size_of::<T::Archived>();
        self.write(slice::from_raw_parts(data, len))?;
        Ok(pos)
    }

    /// Archives the given object and returns the position it was archived at.
    fn serialize_value<T: Serialize<Self>>(&mut self, value: &T) -> Result<usize, Self::Error> {
        let resolver = value.serialize(self)?;
        self.align_for::<T::Archived>()?;
        unsafe { self.resolve_aligned(value, resolver) }
    }

    /// Resolves the given reference with its resolver and writes the archived
    /// reference.
    ///
    /// Returns the position of the written archived reference.
    ///
    /// # Safety
    ///
    /// This is only safe to call when the serializer is already aligned for the
    /// archived reference of the given type.
    unsafe fn resolve_unsized_aligned<T: ArchiveUnsized + ?Sized>(
        &mut self,
        value: &T,
        to: usize,
        metadata_resolver: T::MetadataResolver,
    ) -> Result<usize, Self::Error> {
        let from = self.pos();
        debug_assert!(from & (mem::align_of::<RelPtr<T::Archived>>() - 1) == 0);
        let rel_ptr = value.resolve_unsized(from, to, metadata_resolver);
        let data = (&rel_ptr as *const RelPtr<T::Archived>).cast::<u8>();
        let len = mem::size_of::<RelPtr<T::Archived>>();
        self.write(slice::from_raw_parts(data, len))?;
        Ok(from)
    }

    /// Archives a reference to the given object and returns the position it was
    /// archived at.
    fn serialize_unsized_value<T: SerializeUnsized<Self> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<usize, Self::Error> {
        let to = value.serialize_unsized(self)?;
        let metadata_resolver = value.serialize_metadata(self)?;
        self.align_for::<RelPtr<T::Archived>>()?;
        unsafe { self.resolve_unsized_aligned(value, to, metadata_resolver) }
    }
}

/// A serializer that can seek to an absolute position.
pub trait SeekSerializer: Serializer {
    /// Seeks the serializer to the given absolute position.
    fn seek(&mut self, pos: usize) -> Result<(), Self::Error>;

    /// Archives the given value at the nearest available position. If the
    /// serializer is already aligned, it will archive it at the current position.
    fn archive_root<T: Serialize<Self>>(&mut self, value: &T) -> Result<usize, Self::Error> {
        self.align_for::<T::Archived>()?;
        let pos = self.pos();
        self.seek(pos + mem::size_of::<T::Archived>())?;
        let resolver = value.serialize(self)?;
        self.seek(pos)?;
        unsafe {
            self.resolve_aligned(value, resolver)?;
        }
        Ok(pos)
    }

    /// Archives a reference to the given value at the nearest available
    /// position. If the serializer is already aligned, it will archive it at the
    /// current position.
    fn archive_ref_root<T: SerializeUnsized<Self> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<usize, Self::Error>
    where
        T::Metadata: Serialize<Self>,
        T::Archived: ArchivePointee<ArchivedMetadata = Archived<T::Metadata>>,
    {
        self.align_for::<RelPtr<T::Archived>>()?;
        let pos = self.pos();
        self.seek(pos + mem::size_of::<RelPtr<T::Archived>>())?;
        let to = value.serialize_unsized(self)?;
        let metadata_resolver = value.serialize_metadata(self)?;
        self.seek(pos)?;
        unsafe { self.resolve_unsized_aligned(value, to, metadata_resolver) }
    }
}

/// A serializer that supports serializing shared memory.
///
/// This serializer is required by shared pointers to serialize.
pub trait SharedSerializer: Serializer {
    /// Archives the given shared value and returns its position. If the value
    /// has already been serialized then it returns the position of the
    /// previously serialized value.
    fn archive_shared<T: SerializeUnsized<Self> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<usize, Self::Error>;
}
