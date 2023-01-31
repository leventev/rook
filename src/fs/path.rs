use core::ops::{Deref, Index, IndexMut, Range};

use alloc::{boxed::Box, fmt, string::String};

/// Represents a shared reference to a part(or total) of a path owned by a `PathOwned`
#[derive(Debug, PartialEq)]
pub struct Path<'a>(pub &'a [String]);

/// Represents a path used by the filesystem
#[derive(Debug)]
pub struct PathOwned(pub Box<[String]>);

impl Deref for PathOwned {
    type Target = [String];
    fn deref(&self) -> &Self::Target {
        &(self.0[..])
    }
}

impl<'a> Deref for Path<'a> {
    type Target = [String];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> PathOwned {
    pub fn as_path_ref(&'a self) -> Path<'a> {
        Path(&self.0[..])
    }
}

impl<'a> Path<'a> {
    pub fn new(path: &'a [String]) -> Path<'a> {
        Path(&path)
    }
}

impl<'a> fmt::Display for Path<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "/")?;
        let mut it = self.iter().peekable();
        while let Some(part) = it.next() {
            write!(f, "{}", part)?;
            if it.peek().is_some() {
                write!(f, "/")?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for PathOwned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Path(&self.0))
    }
}

impl Index<usize> for PathOwned {
    type Output = String;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl Index<Range<usize>> for PathOwned {
    type Output = [String];
    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for PathOwned {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}
