use std::{fmt::Display, time::Duration};

use rkyv::Archive;

#[derive(Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Eq)]
pub struct UpdateBehavior {
    pub event: UpdateEvent,
    pub debounce: Option<Debounce>,
    pub throttle: Option<Throttle>,
}

impl UpdateBehavior {
    pub fn default_with_event(event: UpdateEvent) -> Self {
        Self {
            event,
            debounce: Some(Debounce {
                duration: Duration::from_millis(100),
                leading: false,
                notrailing: false,
            }),
            throttle: Some(Throttle {
                duration: Duration::from_millis(100),
                noleading: false,
                trailing: false,
            }),
        }
    }

    pub fn new(event: UpdateEvent, debounce: Option<Debounce>, throttle: Option<Throttle>) -> Self {
        Self {
            event,
            debounce,
            throttle,
        }
    }
}

impl Default for UpdateBehavior {
    fn default() -> Self {
        Self {
            event: UpdateEvent::Change,
            debounce: Some(Debounce {
                duration: Duration::from_millis(100),
                leading: false,
                notrailing: false,
            }),
            throttle: Some(Throttle {
                duration: Duration::from_millis(100),
                noleading: false,
                trailing: false,
            }),
        }
    }
}

impl Display for UpdateBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let debounce = match &self.debounce {
            Some(v) => &format!(
                "__debounce.{}ms{}{}",
                v.duration.as_millis(),
                v.leading.then_some(".leading").unwrap_or_default(),
                v.notrailing.then_some(".notrailing").unwrap_or_default(),
            ),
            None => "",
        };
        let throttle = match &self.throttle {
            Some(v) => &format!(
                "__throttle.{}ms{}{}",
                v.duration.as_millis(),
                v.noleading.then_some(".noleading").unwrap_or_default(),
                v.trailing.then_some(".trailing").unwrap_or_default(),
            ),
            None => "",
        };
        write!(f, "{}{}{}", self.event, debounce, throttle)
    }
}

#[derive(Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Eq)]
pub struct Debounce {
    duration: Duration,
    leading: bool,
    notrailing: bool,
}

impl From<Duration> for Debounce {
    fn from(value: Duration) -> Self {
        Self {
            duration: value,
            leading: false,
            notrailing: false,
        }
    }
}

#[derive(Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Eq)]
pub struct Throttle {
    duration: Duration,
    noleading: bool,
    trailing: bool,
}

impl From<Duration> for Throttle {
    fn from(value: Duration) -> Self {
        Self {
            duration: value,
            noleading: false,
            trailing: false,
        }
    }
}

#[derive(Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Eq)]
pub enum UpdateEvent {
    Blur,
    Change,
    ContextMenu,
    Focus,
    Input,
    Invalid,
    Reset,
    Search,
    Select,
    Submit,
}

impl Display for UpdateEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Blur => {
                    "blur"
                }
                Self::Change => {
                    "change"
                }
                Self::ContextMenu => {
                    "contextmenu"
                }
                Self::Focus => {
                    "focus"
                }
                Self::Input => {
                    "input"
                }
                Self::Invalid => {
                    "invalid"
                }
                Self::Reset => {
                    "reset"
                }
                Self::Search => {
                    "search"
                }
                Self::Select => {
                    "select"
                }
                Self::Submit => {
                    "submit"
                }
            }
        )
    }
}
