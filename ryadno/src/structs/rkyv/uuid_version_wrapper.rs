use rkyv::{
    Archive, Deserialize, Serialize,
    rancor::{Fallible, Source},
    ser::{Allocator, Writer},
};
use uuid::Version;

#[derive(Debug, PartialEq, Clone)]
pub struct UUIDVersionWrapper(pub Version);

impl Archive for UUIDVersionWrapper {
    type Archived = u8;
    type Resolver = ();

    fn resolve(&self, _: Self::Resolver, out: rkyv::Place<Self::Archived>) {
        let num = self.0 as u8;
        out.write(num);
    }
}

impl<S> Serialize<S> for UUIDVersionWrapper
where
    S: Fallible + Allocator + Writer + ?Sized,
    S::Error: Source,
{
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<UUIDVersionWrapper, D> for u8 {
    fn deserialize(&self, _: &mut D) -> Result<UUIDVersionWrapper, D::Error> {
        let version = match *self as usize {
            1 => Version::Mac,
            2 => Version::Dce,
            3 => Version::Md5,
            4 => Version::Random,
            5 => Version::Sha1,
            6 => Version::SortMac,
            7 => Version::SortRand,
            8 => Version::Custom,
            9 => Version::Max,
            _ => Version::Nil,
        };

        Ok(UUIDVersionWrapper(version))
    }
}

impl From<Version> for UUIDVersionWrapper {
    fn from(value: Version) -> Self {
        Self(value)
    }
}
