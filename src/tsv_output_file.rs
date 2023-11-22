use std::fs::File;

use super::*;

pub trait TsvOutputFile<P: Period> {
    fn add_headers(
        &mut self,
        categories: &[IssueCategory],
        category_to_labels: &HashMap<IssueCategory, String>,
    ) -> std::io::Result<()>;

    fn add_row(
        &mut self,
        period: &P,
        period_data: &PeriodData,
        categories: &[IssueCategory],
    ) -> std::io::Result<()>;
}

pub struct PeriodStatsFile<F: Fn(&PeriodData, &IssueCategory) -> i64> {
    file: File,
    get_value_fn: F,
}

impl<F: Fn(&PeriodData, &IssueCategory) -> i64> PeriodStatsFile<F> {
    pub fn new(output_path: &Path, get_value_fn: F) -> Box<Self> {
        let file = File::create(output_path).unwrap();
        Box::new(Self { file, get_value_fn })
    }
}

impl<P: Period, F: Fn(&PeriodData, &IssueCategory) -> i64> TsvOutputFile<P> for PeriodStatsFile<F> {
    fn add_headers(
        &mut self,
        categories: &[IssueCategory],
        category_to_labels: &HashMap<IssueCategory, String>,
    ) -> std::io::Result<()> {
        // let prefix = match self.counter_to_use {
        //     Counter::Opened => "Opened ",
        //     Counter::Closed => "Closed ",
        // };

        write!(self.file, "{}", P::STRING)?;
        for category in categories {
            write!(
                self.file,
                "\tTODO{{prefix}}{category}{}",
                category_to_labels
                    .get(category)
                    .map(|labels| format!(" ({labels})"))
                    .unwrap_or_default()
            )?;
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
            let value = (self.get_value_fn)(period_data, category);
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
    fn add_headers(
        &mut self,
        categories: &[IssueCategory],
        category_to_labels: &HashMap<IssueCategory, String>,
    ) -> std::io::Result<()> {
        write!(self.file, "{}", P::STRING)?;
        for category in categories {
            write!(
                self.file,
                "\tOpen {category}{}",
                category_to_labels
                    .get(category)
                    .map(|labels| format!(" ({labels})"))
                    .unwrap_or_default()
            )?;
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
