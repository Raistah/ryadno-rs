use std::fmt::Display;

use serde::Serialize;
use serde_json::{Map, Value, json};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DataPath {
    segments: Vec<Segment>,
    is_absolute: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum Segment {
    Index(usize),
    Key(String),
    StepBack,
}

/// Enum used in **DataPath.set_value()** to specify behaviour
pub enum ValueUpdateStrategy {
    /// Will incert or update value only if every parent is present and has correct type, and/or provided array index is in a range of single push
    Strict,
    ///	Will create new parent keys, change parent type if needed, array item always will be added, but with different index if out of range
    Flex,
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

    pub fn set_value(
        &self,
        data: &mut Value,
        value: Value,
        strategy: ValueUpdateStrategy,
    ) -> Option<DataPath> {
        let mut iter = self.segments.iter().peekable();
        let mut found_value: &mut Value = data;
        let mut final_data_path: DataPath = DataPath::new();

        match strategy {
            ValueUpdateStrategy::Strict => {
                while let Some(segment) = iter.next() {
                    let next = iter.peek();
                    match segment {
                        Segment::Index(index) => match found_value {
                            Value::Array(item) => {
                                let len = item.len();
                                final_data_path = final_data_path.push(segment.clone());
                                if *index <= len - 1 {
                                    found_value = item.get_mut(*index).unwrap();
                                    continue;
                                } else if len == *index {
                                    match next {
                                        Some(Segment::Index(_)) => {
                                            item.push(Value::Array(vec![]));
                                            found_value = item.get_mut(len).unwrap();
                                            continue;
                                        }
                                        Some(Segment::Key(_)) => {
                                            item.push(Value::Object(Map::new()));
                                            found_value = item.get_mut(len).unwrap();
                                            continue;
                                        }
                                        Some(Segment::StepBack) => {}
                                        None => {
                                            item.push(Value::Null);
                                            found_value = item.get_mut(len).unwrap();
                                            continue;
                                        }
                                    }
                                }
                                return None;
                            }
                            Value::Object(_) => {
                                return None;
                            }
                            _ => {
                                if next == None {
                                    break;
                                }
                                return None;
                            }
                        },
                        Segment::Key(key) => match found_value {
                            Value::Array(_) => {
                                return None;
                            }
                            Value::Object(item) => {
                                final_data_path = final_data_path.push(segment.clone());
                                if let Some(v) = item.get_mut(key) {
                                    found_value = v;
                                    continue;
                                }

                                return None;
                            }
                            _ => {
                                if next == None {
                                    break;
                                }
                                return None;
                            }
                        },
                        _ => (),
                    }
                }
            }
            ValueUpdateStrategy::Flex => {
                while let Some(segment) = iter.next() {
                    let next = iter.peek();
                    match segment {
                        Segment::Index(index) => match found_value {
                            Value::Array(item) => {
                                let len = item.len();
                                if *index <= len - 1 {
                                    final_data_path = final_data_path.push(segment.clone());
                                    found_value = item.get_mut(*index).unwrap();
                                    continue;
                                } else {
                                    final_data_path = final_data_path.push(Segment::Index(len));
                                    match next {
                                        Some(Segment::Index(_)) => {
                                            item.push(Value::Array(vec![]));
                                            found_value = item.get_mut(len).unwrap();
                                            continue;
                                        }
                                        Some(Segment::Key(_)) => {
                                            item.push(Value::Object(Map::new()));
                                            found_value = item.get_mut(len).unwrap();
                                            continue;
                                        }
                                        Some(Segment::StepBack) => {}
                                        None => {
                                            item.push(Value::Null);
                                            found_value = item.get_mut(len).unwrap();
                                            continue;
                                        }
                                    }
                                }
                                return None;
                            }
                            Value::Object(_) => match next {
                                Some(Segment::StepBack) => {}
                                _ => {
                                    *found_value = Value::Array(vec![Value::Null]);
                                    found_value = found_value.get_mut(0).unwrap();
                                    final_data_path = final_data_path.push(Segment::Index(0));
                                    continue;
                                }
                            },
                            _ => {
                                *found_value = Value::Array(vec![Value::Null]);
                                found_value = found_value.get_mut(0).unwrap();
                                final_data_path = final_data_path.push(Segment::Index(0));
                                continue;
                            }
                        },
                        Segment::Key(key) => match found_value {
                            Value::Array(_) => match next {
                                Some(Segment::StepBack) => {}
                                _ => {
                                    *found_value = json!({
                                        key: null
                                    });
                                    found_value = found_value.get_mut(key).unwrap();
                                    final_data_path = final_data_path.push(segment.clone());
                                    continue;
                                }
                            },
                            Value::Object(item) => {
                                final_data_path = final_data_path.push(segment.clone());
                                found_value = item.entry(key.clone()).or_insert(Value::Null);
                            }
                            _ => {
                                *found_value = json!({
                                    key: null
                                });
                                found_value = found_value.get_mut(key).unwrap();
                                final_data_path = final_data_path.push(segment.clone());
                                continue;
                            }
                        },
                        _ => (),
                    }
                }
            }
        }

        *found_value = value;

        Some(final_data_path)
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

    use crate::structs::data_path::{DataPath, ValueUpdateStrategy};

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

    #[test]
    fn test_data_setting() {
        // Strict, Key exists, Success
        let mut test_data_1 = json!({
            "item1": false,
        });
        let test_dp_1 = DataPath::from("item1".to_string());
        let test_fdp_1 = test_dp_1.set_value(
            &mut test_data_1,
            Value::Bool(true),
            ValueUpdateStrategy::Strict,
        );
        assert_eq!(test_fdp_1, Some(DataPath::from("item1".to_string())));
        assert_eq!(
            test_data_1,
            json!({
                "item1": true,
            })
        );

        // Strict, Key exists, Failed, None returned and nothing changed in value
        let mut test_data_2 = json!({
            "item1": false,
        });
        let test_dp_2 = DataPath::from("item2".to_string());
        let test_fdp_2 = test_dp_2.set_value(
            &mut test_data_2,
            Value::Bool(true),
            ValueUpdateStrategy::Strict,
        );
        assert_eq!(test_fdp_2, None);
        assert_eq!(
            test_data_2,
            json!({
                "item1": false,
            })
        );

        // Strict, One index already exists one in one-push range, Success
        let mut test_data_3 = json!(["item1", "item2", "item3",]);
        let test_dp_3 = DataPath::from("[1]".to_string());
        let test_fdp_3 = test_dp_3.set_value(
            &mut test_data_3,
            Value::String("item4".to_string()),
            ValueUpdateStrategy::Strict,
        );
        assert_eq!(test_fdp_3, Some(DataPath::from("[1]".to_string())));
        assert_eq!(test_data_3, json!(["item1", "item4", "item3",]));
        let test_dp_3 = DataPath::from("[3]".to_string());
        let test_fdp_3 = test_dp_3.set_value(
            &mut test_data_3,
            Value::String("item5".to_string()),
            ValueUpdateStrategy::Strict,
        );
        assert_eq!(test_fdp_3, Some(DataPath::from("[3]".to_string())));
        assert_eq!(test_data_3, json!(["item1", "item4", "item3", "item5",]));

        // Strict, Index not exists and out of one-push range, Failed
        let mut test_data_4 = json!(["item1", "item2", "item3",]);
        let test_dp_4 = DataPath::from("[4]".to_string());
        let test_fdp_4 = test_dp_4.set_value(
            &mut test_data_4,
            Value::String("item4".to_string()),
            ValueUpdateStrategy::Strict,
        );
        assert_eq!(test_fdp_4, None);
        assert_eq!(test_data_4, json!(["item1", "item2", "item3",]));

        // Strict, Complex nesting, Success
        let mut test_data_5 = json!([
            {},
            {
                "item1": {
                    "item1": [],
                    "item2": [],
                    "item3": [
                        "item1",
                        "item2",
                        "item3",
                    ],
                }
            },
        ]);
        let test_dp_5 = DataPath::from("[1]/item1/item3/[3]".to_string());
        let test_fdp_5 = test_dp_5.set_value(
            &mut test_data_5,
            Value::String("item4".to_string()),
            ValueUpdateStrategy::Strict,
        );
        assert_eq!(
            test_fdp_5,
            Some(DataPath::from("[1]/item1/item3/[3]".to_string()))
        );
        assert_eq!(
            test_data_5,
            json!([
                {},
                {
                    "item1": {
                        "item1": [],
                        "item2": [],
                        "item3": [
                            "item1",
                            "item2",
                            "item3",
                            "item4",
                        ],
                    }
                },
            ])
        );

        // Strict, Complex nesting, Failed
        let mut test_data_6 = json!([
            {},
            {
                "item1": {
                    "item1": [],
                    "item2": [],
                    "item3": [
                        "item1",
                        "item2",
                        "item3",
                    ],
                }
            },
        ]);
        let test_dp_6 = DataPath::from("[1]/item1/item3/[4]".to_string());
        let test_fdp_6 = test_dp_6.set_value(
            &mut test_data_6,
            Value::String("item4".to_string()),
            ValueUpdateStrategy::Strict,
        );
        assert_eq!(test_fdp_6, None);
        assert_eq!(
            test_data_6,
            json!([
                {},
                {
                    "item1": {
                        "item1": [],
                        "item2": [],
                        "item3": [
                            "item1",
                            "item2",
                            "item3",
                        ],
                    }
                },
            ])
        );
        let test_dp_6 = DataPath::from("[1]/item2/item3/[4]".to_string());
        let test_fdp_6 = test_dp_6.set_value(
            &mut test_data_6,
            Value::String("item4".to_string()),
            ValueUpdateStrategy::Strict,
        );
        assert_eq!(test_fdp_6, None);
        assert_eq!(
            test_data_6,
            json!([
                {},
                {
                    "item1": {
                        "item1": [],
                        "item2": [],
                        "item3": [
                            "item1",
                            "item2",
                            "item3",
                        ],
                    }
                },
            ])
        );

        // Flex, Key not exists, Success
        let mut test_data_7 = json!({
            "item1": false,
        });
        let test_dp_7 = DataPath::from("item2".to_string());
        let test_fdp_7 = test_dp_7.set_value(
            &mut test_data_7,
            Value::Bool(true),
            ValueUpdateStrategy::Flex,
        );
        assert_eq!(test_fdp_7, Some(DataPath::from("item2".to_string())));
        assert_eq!(
            test_data_7,
            json!({
                "item1": false,
                "item2": true,
            })
        );

        // Flex, One index already exists one out of one-push range, Success
        let mut test_data_8 = json!(["item1", "item2", "item3"]);
        let test_dp_8 = DataPath::from("[1]".to_string());
        let test_fdp_8 = test_dp_8.set_value(
            &mut test_data_8,
            Value::String("item4".to_string()),
            ValueUpdateStrategy::Flex,
        );
        assert_eq!(test_fdp_8, Some(DataPath::from("[1]".to_string())));
        assert_eq!(test_data_8, json!(["item1", "item4", "item3"]));
        let test_dp_8 = DataPath::from("[5]".to_string());
        let test_fdp_8 = test_dp_8.set_value(
            &mut test_data_8,
            Value::String("item5".to_string()),
            ValueUpdateStrategy::Flex,
        );
        assert_eq!(test_fdp_8, Some(DataPath::from("[3]".to_string())));
        assert_eq!(test_data_8, json!(["item1", "item4", "item3", "item5"]));

        // Flex, Parent has different type (obj to arr), Success
        let mut test_data_9 = json!({
            "item1": {
                "item1": {
                    "item1": ["item1"]
                }
            }
        });
        let test_dp_9 = DataPath::from("item1/[2]/[1]/item1".to_string());
        let test_fdp_9 = test_dp_9.set_value(&mut test_data_9, json!(1), ValueUpdateStrategy::Flex);
        println!("{:?}: {:?}", test_fdp_9, test_data_9);
        assert_eq!(
            test_fdp_9,
            Some(DataPath::from("item1/[0]/[0]/item1".to_string()))
        );
        assert_eq!(
            test_data_9,
            json!({
                "item1": [
                    [
                        {
                            "item1": 1
                        }
                    ]
                ]
            })
        );

        // Flex, Parent has different type (arr to obj), Success
        let mut test_data_10 = json!({
            "item1": [
                [
                    [
                        "item1"
                    ]
                ]
            ]
        });
        let test_dp_10 = DataPath::from("item1/item1/item1/item1".to_string());
        let test_fdp_10 =
            test_dp_10.set_value(&mut test_data_10, json!(1), ValueUpdateStrategy::Flex);
        println!("{:?}: {:?}", test_fdp_10, test_data_10);
        assert_eq!(
            test_fdp_10,
            Some(DataPath::from("item1/item1/item1/item1".to_string()))
        );
        assert_eq!(
            test_data_10,
            json!({
                "item1": {
                    "item1": {
                        "item1": {
                            "item1": 1
                        }
                    }
                }
            })
        );
    }
}
