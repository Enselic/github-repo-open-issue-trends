use std::fmt::Display;

use chrono::{Utc, Datelike};

trait Period {}

impl Period for Month {}

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
