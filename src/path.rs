#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    segs: Vec<String>,
}

impl Path {
    pub fn root() -> Path {
        Path {
            segs: Vec::new(),
        }
    }

    pub fn is_root(&self) -> bool {
        self.segs.is_empty()
    }

    fn check_seg_valid(seg: &str) -> bool {
        if seg.is_empty() {
            return false;
        }
        for c in seg.chars() {
            if c == '/' || c == '\\' || c == ':' || c == '*' || c == '?' || c == '"' || c == '<' || c == '>' || c == '|' {
                return false;
            }
        }
        true
    }

    pub fn from_str(path: &str) -> Option<Path> {
        if !path.starts_with('/') {
            return None;
        }

        if path.len() == 1 {
            return Some(Path {
                segs: Vec::new(),
            });
        }

        let (_l1, path) = path.split_at(1);

        let mut segs = Vec::new();
        for seg in path.split('/') {
            if !Self::check_seg_valid(seg) {
                return None;
            }
            segs.push(seg.to_string());
        }
        Some(Path {
            segs,
        })
    }

    pub fn to_str(&self) -> String {
        let mut path = String::new();
        path.push('/');
        for seg in &self.segs {
            path.push_str(seg);
            path.push('/');
        }
        if path.len() > 1 {
            path.pop();
        }
        path
    }

    pub fn segs(&self) -> &Vec<String> {
        &self.segs
    }

    pub fn iter(&self) -> std::slice::Iter<String> {
        self.segs.iter()
    }

    pub fn into_iter(self) -> std::vec::IntoIter<String> {
        self.segs.into_iter()
    }

    pub fn parent(mut self) -> Option<Path> {
        if self.segs.is_empty() {
            return None;
        }
        if let Some(_) = self.segs.pop() {
            Some(self)
        } else {
            None
        }
    }

    pub fn push(&mut self, seg: String) {
        if !Self::check_seg_valid(&seg) {
            panic!("cannot push {}", seg);
        }

        self.segs.push(seg);
    }

    pub fn move_push(mut self, seg: String) -> Path {
        self.push(seg);
        self
    }

    pub fn current(&self) -> Option<&String> {
        self.segs.last()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path() {
        let path = Path::from_str("/a/b/c").unwrap();
        assert_eq!(path.to_str(), "/a/b/c");
        assert_eq!(path.segs(), &vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }

    #[should_panic]
    #[test]
    fn test_path_invalid() {
        Path::from_str("//").unwrap();
    }

    #[test]
    fn test_split() {
        let path = Path::from_str("/a/b/c").unwrap();
        let mut iter = path.into_iter();
        assert_eq!(iter.next(), Some("a".to_string()));
        assert_eq!(iter.next(), Some("b".to_string()));
        assert_eq!(iter.next(), Some("c".to_string()));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_root() {
        let path = Path::root();
        assert_eq!(path.to_str(), "/");

        let path = Path::from_str("/").unwrap();
        assert_eq!(path.to_str(), "/");
    }
}