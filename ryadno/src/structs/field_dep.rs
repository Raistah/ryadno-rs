use rkyv::{Archive, Deserialize, Serialize};

use crate::structs::data_path::{self, DataPath};

#[derive(Archive, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct FieldDep {
    segments: Vec<Segment>,
}

#[derive(Archive, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Segment {
    IndexWildcard,
    Index(usize),
    Key(String),
}

impl From<&str> for FieldDep {
    fn from(path: &str) -> Self {
        Self {
            segments: path
                .trim_matches('/')
                .split('/')
                .into_iter()
                .filter_map(|segment| {
                    if segment == "*" {
                        return Some(Segment::IndexWildcard);
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
        }
    }
}

impl FieldDep {
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    pub fn includes(&self, data_path: &DataPath) -> bool {
        if self.len() > data_path.len() {
            return false;
        }

        for (index, segment) in self.segments.iter().enumerate() {
            match data_path.get(index) {
                Some(data_path::Segment::Index(pv)) => match segment {
                    Segment::IndexWildcard => (),
                    Segment::Index(dv) => {
                        if dv != pv {
                            return false;
                        }
                    }
                    Segment::Key(_) => return false,
                },
                Some(data_path::Segment::Key(pv)) => match segment {
                    Segment::IndexWildcard => return false,
                    Segment::Index(_) => return false,
                    Segment::Key(dv) => {
                        if dv != pv {
                            return false;
                        }
                    }
                },
                _ => return false,
            }
        }

        return true;
    }
}

#[cfg(test)]
mod test {
    use crate::structs::{data_path::DataPath, field_dep::FieldDep};

    #[test]
    fn test_dependency_includes() {
        let dep = FieldDep::from("item1/*/item2/*");
        assert_eq!(dep.includes(&DataPath::from("item1/[2]/item2/[0]")), true);

        let dep = FieldDep::from("item1/[1]/item2/*");
        assert_eq!(dep.includes(&DataPath::from("item1/[2]/item2/[0]")), false);

        let dep = FieldDep::from("item1/*/item3");
        assert_eq!(dep.includes(&DataPath::from("item1/[0]/item2/[0]")), false);

        let dep = FieldDep::from("item1/*/item2");
        assert_eq!(dep.includes(&DataPath::from("item1/[0]/item2/item3")), true);

        let dep = FieldDep::from("item1/*/*/*");
        assert_eq!(dep.includes(&DataPath::from("item1/[0]/[1]/[2]/item2")), true);
    }
}
