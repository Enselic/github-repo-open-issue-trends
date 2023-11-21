use std::fmt::{Debug, Display};

use chrono::{Datelike, Utc};

pub trait Period:
    Debug + Display + Ord + std::hash::Hash + Eq + std::convert::From<chrono::DateTime<chrono::Utc>>
{
}

#[derive(Debug, Hash, Eq, PartialEq, PartialOrd, Ord, Copy, Clone)]
pub struct Month {
    year: i32,
    month: u32,
}

impl From<chrono::DateTime<Utc>> for Month {
    fn from(value: chrono::DateTime<Utc>) -> Self {
        Month {
            year: value.year(),
            month: value.month(),
        }
    }
}

impl Period for Month {}

impl Display for Month {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}",
            self.year,
            match self.month {
                1 => "Jan",
                2 => "Feb",
                3 => "Mar",
                4 => "Apr",
                5 => "May",
                6 => "Jun",
                7 => "Jul",
                8 => "Aug",
                9 => "Sep",
                10 => "Oct",
                11 => "Nov",
                12 => "Dec",
                _ => unreachable!(),
            }
        )
    }
}

#[derive(Debug, Hash, Eq, PartialEq, PartialOrd, Ord, Copy, Clone)]
pub struct TwoMonths {
    year: i32,
    month_pair: u32,
}

impl From<chrono::DateTime<Utc>> for TwoMonths {
    fn from(value: chrono::DateTime<Utc>) -> Self {
        Self {
            year: value.year(),
            month_pair: (value.month() - 1) / 2,
        }
    }
}

impl Display for TwoMonths {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}",
            self.year,
            match self.month_pair {
                0 => "Jan-Feb",
                1 => "Mar-Apr",
                2 => "May-Jun",
                3 => "Jul-Aug",
                4 => "Sep-Oct",
                5 => "Nov-Dec",
                _ => unreachable!(),
            }
        )
    }
}

impl Period for TwoMonths {}

#[derive(Debug, Hash, Eq, PartialEq, PartialOrd, Ord, Copy, Clone)]
pub struct Quarter {
    year: i32,
    quarter: u32,
}

impl From<chrono::DateTime<Utc>> for Quarter {
    fn from(value: chrono::DateTime<Utc>) -> Self {
        Self {
            year: value.year(),
            quarter: (value.month() - 1) / 3,
        }
    }
}

impl Display for Quarter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}",
            self.year,
            match self.quarter {
                0 => "Q1",
                1 => "Q2",
                2 => "Q3",
                3 => "Q4",
                _ => unreachable!(),
            }
        )
    }
}

impl Period for Quarter {}
