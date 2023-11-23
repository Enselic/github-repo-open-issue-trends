use std::fs::File;

use super::*;

pub trait TsvColumns<P: Period> {
    fn add_headers(
        &mut self,
        w: &mut File,
        categories: &[IssueCategory],
        category_to_labels: &HashMap<IssueCategory, String>,
    ) -> std::io::Result<()>;

    fn add_row(
        &mut self,
        w: &mut File,
        period_data: &PeriodData,
        categories: &[IssueCategory],
    ) -> std::io::Result<()>;
}

pub struct PeriodColumns<F: Fn(&PeriodData, &IssueCategory) -> i64> {
    header_prefix: String,
    get_value_fn: F,
}

impl<F: Fn(&PeriodData, &IssueCategory) -> i64> PeriodColumns<F> {
    pub fn new(header_prefix: String, get_value_fn: F) -> Box<Self> {
        Box::new(Self {
            header_prefix,
            get_value_fn,
        })
    }
}

impl<P: Period, F: Fn(&PeriodData, &IssueCategory) -> i64> TsvColumns<P> for PeriodColumns<F> {
    fn add_headers(
        &mut self,
        w: &mut File,
        categories: &[IssueCategory],
        category_to_labels: &HashMap<IssueCategory, String>,
    ) -> std::io::Result<()> {
        for category in categories {
            write!(
                w,
                "\t{}{}{}",
                &self.header_prefix,
                category,
                category_to_labels
                    .get(category)
                    .map(|labels| format!(" ({labels})"))
                    .unwrap_or_default()
            )?;
        }
        Ok(())
    }

    fn add_row(
        &mut self,
        w: &mut File,
        period_data: &PeriodData,
        categories: &[IssueCategory],
    ) -> std::io::Result<()> {
        for category in categories {
            let value = (self.get_value_fn)(period_data, category);
            write!(w, "\t{value}",)?;
        }
        Ok(())
    }
}

pub struct AccumulatedPeriodStatsFile {
    total: HashMap<IssueCategory, i64>,
}

impl AccumulatedPeriodStatsFile {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            total: HashMap::new(),
        })
    }
}

impl<P: Period> TsvColumns<P> for AccumulatedPeriodStatsFile {
    fn add_headers(
        &mut self,
        w: &mut File,
        categories: &[IssueCategory],
        category_to_labels: &HashMap<IssueCategory, String>,
    ) -> std::io::Result<()> {
        for category in categories {
            write!(
                w,
                "\tOpen {category}{}",
                category_to_labels
                    .get(category)
                    .map(|labels| format!(" ({labels})"))
                    .unwrap_or_default()
            )?;
        }
        Ok(())
    }

    fn add_row(
        &mut self,
        w: &mut File,
        period_data: &PeriodData,
        categories: &[IssueCategory],
    ) -> std::io::Result<()> {
        for category in categories {
            let delta = period_data.get(category, Counter::Opened)
                - period_data.get(category, Counter::Closed);
            self.total
                .entry(category.clone())
                .and_modify(|c| *c += delta)
                .or_insert(delta);

            write!(w, "\t{}", self.total.get(category).unwrap())?;
        }
        Ok(())
    }
}
