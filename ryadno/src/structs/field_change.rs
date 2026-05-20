use crate::structs::data_path::DataPath;

#[derive(Debug)]
pub struct FieldChange {
    pub data_path: DataPath,
    pub result: ChangeType,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ChangeType {
    RerenderField{
        selector: String,
        data: Result<String, String>
    },
    OpenModal(Result<String, String>)
}
