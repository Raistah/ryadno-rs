use std::fmt::Display;

use serde_json::Value;

#[derive(Debug, PartialEq, Eq)]
pub struct DataPath {
    segments: Vec<Segment>,
    is_absolute: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Segment {
    Index(usize),
    Key(String),
    StepBack,
}

impl DataPath {
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
            is_absolute: false,
        }
    }

    pub fn is_absoulte(&self) -> bool {
        self.is_absolute
    }

    pub fn absolute(mut self) -> Self {
        self.is_absolute = true;
        self.normalize()
    }

    pub fn relative(mut self) -> Self {
        self.is_absolute = false;
        self.normalize()
    }

    pub fn push(mut self, segment: Segment) -> Self {
        self.segments.push(segment);
        self.normalize()
    }

    pub fn extend(mut self, end: DataPath) -> Self {
        self.segments.extend(end.segments);
        self.normalize()
    }

    pub fn normalize(mut self) -> Self {
        let mut segments: Vec<Segment> = Vec::new();
        let mut has_non_step_back = false;
        for segment in self.segments {
            match segment {
                Segment::StepBack => {
                    if !self.is_absolute && !has_non_step_back {
                        segments.push(Segment::StepBack);
                    } else {
                        segments.pop();
                    }
                }
                Segment::Index(v) => {
                    if !has_non_step_back {
                        has_non_step_back = true;
                    }
                    segments.push(Segment::Index(v));
                }
                Segment::Key(v) => {
                    if !has_non_step_back {
                        has_non_step_back = true;
                    }
                    segments.push(Segment::Key(v));
                }
            };
        }
        self.segments = segments;
        self
    }

    // StepBack is always skiped, because path should be normolized at this point
    pub fn find_value<'a>(&self, value: &'a Value) -> Option<&'a Value> {
        let mut found_value = Some(value);
        for segment in self.segments.iter() {
            match found_value {
                Some(v) => match v {
                    Value::Array(v) => match segment {
                        Segment::StepBack => {}
                        Segment::Index(i) => match v.get(i.to_owned()) {
                            Some(v) => {
                                found_value = Some(v);
                            }
                            None => return None,
                        },
                        Segment::Key(_) => return None,
                    },
                    Value::Object(v) => match segment {
                        Segment::StepBack => {}
                        Segment::Index(_) => return None,
                        Segment::Key(k) => match v.get(k) {
                            Some(v) => {
                                found_value = Some(v);
                            }
                            None => return None,
                        },
                    },
                    _ => return None,
                },
                None => return None,
            }
        }

        found_value
    }
}

impl From<&str> for DataPath {
    fn from(path: &str) -> Self {
        Self {
            segments: path
                .trim_matches('/')
                .split('/')
                .into_iter()
                .filter_map(|segment| {
                    if segment == "." {
                        return None;
                    }
                    if segment == ".." {
                        return Some(Segment::StepBack);
                    }
                    if segment.starts_with('[') && segment.ends_with(']') {
                        match segment.replace("[", "").replace("]", "").parse::<usize>() {
                            Ok(v) => return Some(Segment::Index(v)),
                            Err(_) => return None,
                        }
                    }
                    Some(Segment::Key(segment.to_string()))
                })
                .collect(),
            is_absolute: path.starts_with("/"),
        }
        .normalize()
    }
}

impl From<String> for DataPath {
    fn from(path: String) -> Self {
        Self {
            segments: path
                .trim_matches('/')
                .split('/')
                .into_iter()
                .filter_map(|segment| {
                    if segment == "." {
                        return None;
                    }
                    if segment == ".." {
                        return Some(Segment::StepBack);
                    }
                    if segment.starts_with('[') && segment.ends_with(']') {
                        match segment.replace("[", "").replace("]", "").parse::<usize>() {
                            Ok(v) => return Some(Segment::Index(v)),
                            Err(_) => return None,
                        }
                    }
                    Some(Segment::Key(segment.to_string()))
                })
                .collect(),
            is_absolute: path.starts_with("/"),
        }
        .normalize()
    }
}

impl Display for DataPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}",
            if self.is_absolute { "/" } else { "" },
            self.segments
                .iter()
                .map(|segment| {
                    match segment {
                        Segment::Index(v) => format!("[{}]", v),
                        Segment::Key(v) => v.to_owned(),
                        Segment::StepBack => "..".to_string(),
                    }
                })
                .collect::<Vec<String>>()
                .join("/")
        )
    }
}

#[cfg(test)]
mod test {
    use serde_json::{Value, json};

    use crate::structs::data_path::DataPath;

    #[test]
    fn test_data_path_normalization() {
        assert_eq!(DataPath::from("a/b/c/../../d"), DataPath::from("a/d"));

        assert_eq!(DataPath::from("../a/b/c/../../d"), DataPath::from("../a/d"));

        assert_eq!(
            DataPath::from("../a/b/c/../../d").absolute(),
            DataPath::from("/a/d")
        );
    }

    #[test]
    fn test_data_finding() {
        let value: Value = json!({
            "a": "value a",
            "b": {
                "vec": [
                    "vec 1",
                    "vec 2",
                    "vec 3",
                    "vec 4",
                    "vec 5",
                ],
            },
            "c": [
                {
                    "subVec1": [],
                    "subVec2": [],
                    "subVec3": [
                        "subVec3 1",
                        "subVec3 2",
                        "subVec3 3",
                        "subVec3 4",
                    ],
                },
            ]
        });

        let path1 = DataPath::from("a");
        assert_eq!(
            path1.find_value(&value),
            Some(&Value::String("value a".to_string()))
        );

        let path2 = DataPath::from("/a/d");
        assert_eq!(path2.find_value(&value), None);

        let path3 = DataPath::from("/b/vec/[3]");
        assert_eq!(
            path3.find_value(&value),
            Some(&Value::String("vec 4".to_string()))
        );

        let path4 = DataPath::from("/c/[0]/subVec3/[2]");
        assert_eq!(
            path4.find_value(&value),
            Some(&Value::String("subVec3 3".to_string()))
        );

        let path5 = path4.extend(DataPath::from("../../../../b/vec/[0]"));
        assert_eq!(
            path5.find_value(&value),
            Some(&Value::String("vec 1".to_string()))
        );

        let path6 = DataPath::from("/b/vec");
        assert_eq!(
            path6.find_value(&value),
            Some(&Value::Array(vec![
                Value::String("vec 1".to_string()),
                Value::String("vec 2".to_string()),
                Value::String("vec 3".to_string()),
                Value::String("vec 4".to_string()),
                Value::String("vec 5".to_string()),
            ]))
        );
    }
}
