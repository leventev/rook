use crate::posix::errno::{Errno, ENAMETOOLONG};

pub const PATH_COMPONENT_MAX: usize = 256;
pub const PATH_FULL_MAX: usize = 4096;

#[derive(Debug)]
pub enum PathParseError {
    PathComponentTooLong,
    PathTooLong,
}

impl Into<Errno> for PathParseError {
    fn into(self) -> Errno {
        match self {
            PathParseError::PathComponentTooLong | PathParseError::PathTooLong => ENAMETOOLONG,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Path<'a> {
    buff: &'a str,
    components_left: usize,
}

impl<'a> Path<'a> {
    pub fn new(buff: &'a str) -> Result<Path, PathParseError> {
        assert!(
            buff.starts_with('/'),
            "Paths given to the path parser must be absolute",
        );

        if buff.len() > PATH_FULL_MAX {
            return Err(PathParseError::PathTooLong);
        }

        let mut count = 0;
        for comp in buff.split('/') {
            if comp.is_empty() {
                continue;
            }
            if comp.len() > PATH_COMPONENT_MAX {
                return Err(PathParseError::PathComponentTooLong);
            }

            count += 1;
        }

        Ok(Path {
            buff: &buff[1..],
            components_left: count,
        })
    }

    pub fn components_left(&self) -> usize {
        self.components_left
    }

    pub fn shorten(self, count: usize) -> Path<'a> {
        debug_assert!(count <= self.components_left);
        Path {
            buff: self.buff,
            components_left: count,
        }
    }
}

impl<'a> Iterator for Path<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        if self.components_left == 0 {
            debug_assert!(self.buff.is_empty());
            return None;
        }

        let end = self.buff.find('/').unwrap_or(self.buff.len());

        let segment = &self.buff[..end];
        debug_assert!(segment.len() < PATH_COMPONENT_MAX);

        let next_start_idx = if self.components_left > 1 {
            end + 1
        } else {
            end
        };

        self.buff = &self.buff[next_start_idx..];
        match segment.len() {
            0 => None,
            _ => {
                self.components_left -= 1;
                Some(segment)
            }
        }
    }
}
