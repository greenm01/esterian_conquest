use std::ops::{Add, AddAssign, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Column(pub usize);

impl Column {
    pub const fn as_usize(self) -> usize {
        self.0
    }
}

impl From<usize> for Column {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl Add<usize> for Column {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<usize> for Column {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl Sub<usize> for Column {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl SubAssign<usize> for Column {
    fn sub_assign(&mut self, rhs: usize) {
        self.0 -= rhs;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Row(pub usize);

impl Row {
    pub const fn as_usize(self) -> usize {
        self.0
    }
}

impl From<usize> for Row {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl Add<usize> for Row {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<usize> for Row {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl Sub<usize> for Row {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl SubAssign<usize> for Row {
    fn sub_assign(&mut self, rhs: usize) {
        self.0 -= rhs;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Point {
    pub column: Column,
    pub row: Row,
}

impl Point {
    pub const fn new(column: Column, row: Row) -> Self {
        Self { column, row }
    }

    pub const fn from_usize(column: usize, row: usize) -> Self {
        Self::new(Column(column), Row(row))
    }
}
