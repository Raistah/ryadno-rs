use std::{any::Any, pin::Pin, sync::Arc};

use linkme::distributed_slice;
use rkyv::Archive;

use crate::{
    form::{FormContext, ValueGetter},
    structs::{
        data_path::DataPath,
        rkyv::{
            datetime_wrapper::DateTimeWrapper, uuid_version_wrapper::UUIDVersionWrapper,
            value_wrapper::ValueWrapper,
        },
    },
};

#[distributed_slice]
#[linkme(crate = crate::linkme)]
pub static RYADNO_FIELDS_VALIDATION_CLOUSRES: [(&'static str, ValidationClosure)];
pub type ValidationClosure = for<'a> fn(
    data_path: Arc<DataPath>,
    form_context: Arc<FormContext>,
    runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
    get: ValueGetter<'a>,
) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>>;

#[derive(Archive, rkyv::Serialize, rkyv::Deserialize, Debug, Clone)]
pub enum ValidationRule {
    Required,
    Is(ExpectedType),
    Same(Arc<DataPath>),
    Different(Arc<DataPath>),
    OneOf(Vec<ValueWrapper>),
    NotOneOf(Vec<ValueWrapper>),
    Min(f64),
    Max(f64),
    Between(f64, f64),

    // Bool
    Accepted,
    AcceptedIf {
        data_path: Arc<DataPath>,
        value: ValueWrapper,
    },
    Declined,
    DeclinedIf {
        data_path: Arc<DataPath>,
        value: ValueWrapper,
    },

    // String
    StartsWith(String),
    DoesntStartWith(String),
    EndsWith(String),
    DoesntEndWith(String),
    Email,
    HexColor,
    Ip(Option<IpVersion>),
    MAC,
    JSON,
    Lowercase,
    Uppercase,
    Regex(String),
    NotRegex(String),
    URL,
    ULID,
    UUID(Option<UUIDVersionWrapper>),

    // Number
    Decimal(Option<u8>, Option<u8>),
    Digits(u8),
    MaxDigits(u8),
    MinDigits(u8),
    DigitsBetween(u8, u8),
    GreaterThan(Arc<DataPath>),
    GreaterThanOrEqual(Arc<DataPath>),
    LessThen(Arc<DataPath>),
    LessThenOrEqual(Arc<DataPath>),
    Integer,
    MultipleOf(f64),

    // Array
    Contain(ValueWrapper),
    DoesntContain(ValueWrapper),
    InArray(Arc<DataPath>), // Current item in other field
    Distinct,

    // Dates
    IsDate,
    After(DateTimeWrapper),
    AfterOther(Arc<DataPath>),
    AfterOrEqual(DateTimeWrapper),
    AfterOrEqualToOther(Arc<DataPath>),
    Before(DateTimeWrapper),
    BeforeOther(Arc<DataPath>),
    BeforeOrEqual(DateTimeWrapper),
    BeforeOrEqualToOther(Arc<DataPath>),

    // Files
    // Commented till i figure out how to deal with files
    // IsFile,
    // IsImage,
    // MIMETypes,

    // Custom works in the same way as field closures
    Custom(String),
}

impl PartialEq for ValidationRule {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Required, Self::Required) => true,
            (Self::Is(a), Self::Is(b)) => a == b,
            (Self::Same(a), Self::Same(b)) => a == b,
            (Self::Different(a), Self::Different(b)) => a == b,
            (Self::OneOf(a), Self::OneOf(b)) => a == b,
            (Self::NotOneOf(a), Self::NotOneOf(b)) => a == b,
            (Self::Min(a), Self::Min(b)) => a == b,
            (Self::Max(a), Self::Max(b)) => a == b,
            (Self::Between(a, b), Self::Between(c, d)) => a == c && b == d,

            (Self::Accepted, Self::Accepted) => true,
            (
                Self::AcceptedIf {
                    data_path: a,
                    value: b,
                },
                Self::AcceptedIf {
                    data_path: c,
                    value: d,
                },
            ) => a == c && b == d,
            (Self::Declined, Self::Declined) => true,
            (
                Self::DeclinedIf {
                    data_path: a,
                    value: b,
                },
                Self::DeclinedIf {
                    data_path: c,
                    value: d,
                },
            ) => a == c && b == d,

            (Self::StartsWith(a), Self::StartsWith(b)) => a == b,
            (Self::DoesntStartWith(a), Self::DoesntStartWith(b)) => a == b,
            (Self::EndsWith(a), Self::EndsWith(b)) => a == b,
            (Self::DoesntEndWith(a), Self::DoesntEndWith(b)) => a == b,
            (Self::Email, Self::Email) => true,
            (Self::HexColor, Self::HexColor) => true,
            (Self::Ip(a), Self::Ip(b)) => a == b,
            (Self::MAC, Self::MAC) => true,
            (Self::JSON, Self::JSON) => true,
            (Self::Lowercase, Self::Lowercase) => true,
            (Self::Uppercase, Self::Uppercase) => true,
            (Self::Regex(a), Self::Regex(b)) => a == b,
            (Self::NotRegex(a), Self::NotRegex(b)) => a == b,
            (Self::URL, Self::URL) => true,
            (Self::ULID, Self::ULID) => true,
            (Self::UUID(a), Self::UUID(b)) => a == b,

            (Self::Decimal(a, b), Self::Decimal(c, d)) => a == c && b == d,
            (Self::Digits(a), Self::Digits(b)) => a == b,
            (Self::MaxDigits(a), Self::MaxDigits(b)) => a == b,
            (Self::MinDigits(a), Self::MinDigits(b)) => a == b,
            (Self::DigitsBetween(a, b), Self::DigitsBetween(c, d)) => a == c && b == d,
            (Self::GreaterThan(a), Self::GreaterThan(b)) => a == b,
            (Self::GreaterThanOrEqual(a), Self::GreaterThanOrEqual(b)) => a == b,
            (Self::LessThen(a), Self::LessThen(b)) => a == b,
            (Self::LessThenOrEqual(a), Self::LessThenOrEqual(b)) => a == b,
            (Self::Integer, Self::Integer) => true,
            (Self::MultipleOf(a), Self::MultipleOf(b)) => a == b,

            (Self::Contain(a), Self::Contain(b)) => a == b,
            (Self::DoesntContain(a), Self::DoesntContain(b)) => a == b,
            (Self::InArray(a), Self::InArray(b)) => a == b,
            (Self::Distinct, Self::Distinct) => true,

            (Self::IsDate, Self::IsDate) => true,
            (Self::After(a), Self::After(b)) => a == b,
            (Self::AfterOther(a), Self::AfterOther(b)) => a == b,
            (Self::AfterOrEqual(a), Self::AfterOrEqual(b)) => a == b,
            (Self::AfterOrEqualToOther(a), Self::AfterOrEqualToOther(b)) => a == b,
            (Self::Before(a), Self::Before(b)) => a == b,
            (Self::BeforeOther(a), Self::BeforeOther(b)) => a == b,
            (Self::BeforeOrEqual(a), Self::BeforeOrEqual(b)) => a == b,
            (Self::BeforeOrEqualToOther(a), Self::BeforeOrEqualToOther(b)) => a == b,

            (Self::Custom(a), Self::Custom(b)) => a == b,

            _ => false,
        }
    }
}
impl Eq for ValidationRule {}

#[derive(Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum ExpectedType {
    Null,
    Bool,
    Number,
    String,
    Array,
    Object,
}

#[derive(Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum IpVersion {
    V4,
    V6,
}
