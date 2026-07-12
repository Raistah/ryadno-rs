use rkyv::{
    Archive, Deserialize, Serialize,
    rancor::{Fallible, Source},
    ser::{Allocator, Writer},
    string::{ArchivedString, StringResolver},
};
use serde_json::Value;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ValueWrapper(pub Value);

impl Archive for ValueWrapper {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    fn resolve(&self, resolver: Self::Resolver, out: rkyv::Place<Self::Archived>) {
        let s = self.0.to_string();
        ArchivedString::resolve_from_str(&s, resolver, out);
    }
}

impl<S> Serialize<S> for ValueWrapper
where
    S: Fallible + Allocator + Writer + ?Sized,
    S::Error: Source,
{
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        Self::Archived::serialize_from_str::<_>(self.0.to_string().as_str(), serializer)
    }
}

impl<D: Fallible + ?Sized> Deserialize<ValueWrapper, D> for ArchivedString {
    fn deserialize(&self, _: &mut D) -> Result<ValueWrapper, D::Error> {
        let string = self.to_string();
        let value: serde_json::Value = serde_json::from_str(string.as_str()).unwrap();
        Ok(ValueWrapper(value))
    }
}

impl From<Value> for ValueWrapper {
    fn from(value: Value) -> Self {
        Self(value)
    }
}
