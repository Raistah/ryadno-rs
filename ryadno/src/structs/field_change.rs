use axum::response::sse::Event;

use crate::structs::data_path::DataPath;

#[derive(Debug)]
pub struct FieldChange {
    pub data_path: DataPath,
    pub result: ChangeType,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ChangeType {
    RerenderField {
        selector: String,
        data: Result<String, String>,
    },
    OpenModal(Result<String, String>),
}

impl FieldChange {
    pub fn to_datastar_event(&self, form_uuid: &String) -> Event {
        match &self.result {
            ChangeType::RerenderField { selector, data } => {
                let mut event = Event::default().event("datastar-patch-elements");
                match data {
                    Ok(d) | Err(d) => {
                        event = event.data(format!(
                            concat!("selector {}\n", "mode inner\n", "elements {}"),
                            selector,
                            d.chars()
                                .filter_map(|x| match x {
                                    '\n' => Some(' '),
                                    '\t' => None,
                                    _ => Some(x),
                                })
                                .collect::<String>()
                        ));
                    }
                }
                return event;
            }
            ChangeType::OpenModal(data) => {
                let mut event = Event::default().event("datastar-patch-elements");
                match data {
                    Ok(d) | Err(d) => {
                        event = event.data(format!(
                            concat!("selector {}\n", "mode append\n", "elements {}"),
                            form_uuid,
                            d.chars()
                                .filter_map(|x| match x {
                                    '\n' => Some(' '),
                                    '\t' => None,
                                    _ => Some(x),
                                })
                                .collect::<String>()
                        ));
                    }
                }
                return event;
            }
        }
    }
}
