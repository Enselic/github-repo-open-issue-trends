use std::fs::File;

use super::*;

pub trait TsvColumns<P: Period> {
    fn add_headers(
        &mut self,
        w: &mut impl Write,
        categories: &[IssueCategory],
        category_to_labels: &HashMap<IssueCategory, String>,
    ) -> std::io::Result<()>;

    fn add_row(
        &mut self,
        w: &mut impl Write,
        period: &P,
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
        w: &mut impl Write,
        categories: &[IssueCategory],
        category_to_labels: &HashMap<IssueCategory, String>,
    ) -> std::io::Result<()> {
        write!(w, "{}", P::STRING)?;
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
        writeln!(w)
    }

    fn add_row(
        &mut self,
        w: &mut impl Write,
        period: &P,
        period_data: &PeriodData,
        categories: &[IssueCategory],
    ) -> std::io::Result<()> {
        write!(w, "{period}")?;
        for category in categories {
            let value = (self.get_value_fn)(period_data, category);
            write!(w, "\t{value}",)?;
        }
        writeln!(w)
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

impl<P: Period> TsvColumns<P> for AccumulatedPeriodStatsFile {
    fn add_headers(
        &mut self,
        w: &mut impl Write,
        categories: &[IssueCategory],
        category_to_labels: &HashMap<IssueCategory, String>,
    ) -> std::io::Result<()> {
        write!(w, "{}", P::STRING)?;
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
        writeln!(w)
    }

    fn add_row(
        &mut self,
        w: &mut impl Write,
        period: &P,
        period_data: &PeriodData,
        categories: &[IssueCategory],
    ) -> std::io::Result<()> {
        write!(w, "{period}")?;
        for category in categories {
            let delta = period_data.get(category, Counter::Opened)
                - period_data.get(category, Counter::Closed);
            self.total
                .entry(category.clone())
                .and_modify(|c| *c += delta)
                .or_insert(delta);

            write!(w, "\t{}", self.total.get(category).unwrap())?;
        }
        writeln!(w)
    }
}
