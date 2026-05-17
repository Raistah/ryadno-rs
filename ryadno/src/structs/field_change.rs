use crate::structs::data_path::DataPath;

pub struct FieldChange {
    pub data_path: DataPath,
    pub result: FieldChangeResult
}

pub enum FieldChangeResult {
    Ok(String),
    Err(String)
}
