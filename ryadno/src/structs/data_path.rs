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
        self
    }

    pub fn relative(mut self) -> Self {
        self.is_absolute = false;
        self
    }

    pub fn push(mut self, segment: Segment) -> Self {
        self.segments.push(segment);
        self
    }

    pub fn extend(mut self, end: DataPath) -> Self {
        self.segments.extend(end.segments);
        self
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

    // It is probably imposible to find value if path is relative.
    // So i need to return Option
    // I need to have recursive function
    // I need to decide how to handle StepBack
    pub fn find_value(&self, value: &Value) -> Option<&Value> {
        todo!()
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
    use crate::structs::data_path::DataPath;

    #[test]
    fn test_data_path_normalization() {
        assert_eq!(
            DataPath::from("a/b/c/../../d").normalize(),
            DataPath::from("a/d")
        );

        assert_eq!(
            DataPath::from("../a/b/c/../../d").normalize(),
            DataPath::from("../a/d")
        );

        assert_eq!(
            DataPath::from("../a/b/c/../../d").absolute().normalize(),
            DataPath::from("/a/d")
        );
    }
}
