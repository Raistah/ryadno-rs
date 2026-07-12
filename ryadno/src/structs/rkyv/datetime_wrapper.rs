use chrono::{DateTime, Utc};
use rkyv::{
    Archive, Deserialize, Serialize,
    rancor::{Fallible, Source},
    ser::{Allocator, Writer},
    string::{ArchivedString, StringResolver},
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DateTimeWrapper(pub DateTime<Utc>);

impl Archive for DateTimeWrapper {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    fn resolve(&self, resolver: Self::Resolver, out: rkyv::Place<Self::Archived>) {
        let s = self.0.to_rfc3339();
        ArchivedString::resolve_from_str(&s, resolver, out);
    }
}

impl<S> Serialize<S> for DateTimeWrapper
where
    S: Fallible + Allocator + Writer + ?Sized,
    S::Error: Source,
{
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        let s = self.0.to_rfc3339();
        Self::Archived::serialize_from_str::<_>(s.as_str(), serializer)
    }
}

impl<D: Fallible + ?Sized> Deserialize<DateTimeWrapper, D> for ArchivedString {
    fn deserialize(&self, _: &mut D) -> Result<DateTimeWrapper, D::Error> {
        let string = self.to_string();
        let dt = DateTime::parse_from_rfc3339(&string)
            .map(|dt| dt.with_timezone(&Utc))
            .expect("Failed to parse archived datetime string");

        Ok(DateTimeWrapper(dt))
    }
}

impl From<DateTime<Utc>> for DateTimeWrapper {
    fn from(value: DateTime<Utc>) -> Self {
        Self(value)
    }
}
