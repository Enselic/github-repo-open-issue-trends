use std::fs::File;

use super::*;

pub trait TsvOutputFile<P: Period> {
    fn add_headers(&mut self, categories: &[IssueCategory]) -> std::io::Result<()>;

    fn add_row(
        &mut self,
        period: &P,
        period_data: &PeriodData,
        categories: &[IssueCategory],
    ) -> std::io::Result<()>;
}

pub struct PeriodStatsFile {
    file: File,
    counter_to_use: Counter,
}

impl PeriodStatsFile {
    pub fn new(output_path: &Path, counter_to_use: Counter) -> std::io::Result<Self> {
        let file = File::create(output_path)?;
        let x = Self {
            file,
            counter_to_use,
        };
        Ok(x)
    }
}

impl<P: Period> TsvOutputFile<P> for PeriodStatsFile {
    fn add_headers(&mut self, categories: &[IssueCategory]) -> std::io::Result<()> {
        let prefix = match self.counter_to_use {
            Counter::Opened => "Opened ",
            Counter::Closed => "Closed ",
        };

        write!(self.file, "Month")?;
        for category in categories {
            write!(self.file, "\t{prefix}{category}")?;
        }
        writeln!(self.file)
    }

    fn add_row(
        &mut self,
        period: &P,
        period_data: &PeriodData,
        categories: &[IssueCategory],
    ) -> std::io::Result<()> {
        write!(self.file, "{period}")?;
        for category in categories {
            let value = period_data.get(category.clone(), self.counter_to_use);
            write!(self.file, "\t{value}",)?;
        }
        writeln!(self.file)
    }
}

pub struct AccumulatedPeriodStatsFile {
    file: File,
    total: HashMap<IssueCategory, i64>,
}

impl AccumulatedPeriodStatsFile {
    pub fn new(output_path: &Path) -> std::io::Result<Self> {
        let file = File::create(output_path)?;
        let x = Self {
            file,
            total: HashMap::new(),
        };
        Ok(x)
    }
}

impl<P: Period> TsvOutputFile<P> for AccumulatedPeriodStatsFile {
    fn add_headers(&mut self, categories: &[IssueCategory]) -> std::io::Result<()> {
        write!(self.file, "Month")?;
        for category in categories {
            write!(self.file, "\tOpen {category}")?;
        }
        writeln!(self.file)
    }

    fn add_row(
        &mut self,
        period: &P,
        period_data: &PeriodData,
        categories: &[IssueCategory],
    ) -> std::io::Result<()> {
        write!(self.file, "{period}")?;
        for category in categories {
            let delta = period_data.get(category.clone(), Counter::Opened)
                - period_data.get(category.clone(), Counter::Closed);
            self.total
                .entry(category.clone())
                .and_modify(|c| *c += delta)
                .or_insert(delta);

            write!(self.file, "\t{}", self.total.get(category).unwrap())?;
        }
        writeln!(self.file)
    }
}
