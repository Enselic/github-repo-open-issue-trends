use std::fmt::Display;
use std::fs::File;
use std::io::Write;
use std::{collections::HashMap, path::PathBuf};

use tracing::*;

mod models;
use models::*;

mod tsv_output_file;
use tsv_output_file::*;

mod period;
use period::*;

mod utils;
use utils::*;

#[derive(clap::Parser, Debug)]
pub struct Args {
    /// The GitHub repo. E.g. "rust-lang/rust"
    repo: String,

    #[arg(long, default_value = "month")]
    /// Whether to use months or weeks as the period.
    period: PeriodEnum,

    /// How many issues to fetch per page.
    #[arg(long, default_value = "10")]
    page_size: i64,

    /// How many pages to fetch.
    #[arg(long, default_value = "2")]
    pages: usize,

    /// Maps a GitHub issue label (e.g. "C-bug") to a category (e.g. "bugs").
    /// Syntax: "label:category". Can be specified multiple times.
    #[arg(short='c', long, value_parser = parse_label_category, value_delimiter = ',')]
    label_category: Vec<(String, String)>,

    /// Path of the output .tsv file
    #[arg(long, default_value = "issues.tsv")]
    tsv_output: PathBuf,

    /// Where to save GitHub GraphQL API responses to save on the rate limit. By
    /// default `~/.cache/enselic/github-repo-open-issues/...` is used.
    #[arg(long)]
    cached_responses_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = <Args as clap::Parser>::parse();

    match args.period {
        PeriodEnum::Month => run_main::<Month>(&args).await,
        PeriodEnum::TwoMonths => run_main::<TwoMonths>(&args).await,
        PeriodEnum::Quarter => run_main::<Quarter>(&args).await,
        PeriodEnum::Year => run_main::<Year>(&args).await,
    }
}

async fn run_main<P: Period>(args: &Args) -> anyhow::Result<()> {
    // Collect data
    let plot_data = collect_data(&args).await;
    let mut sorted_periods = plot_data.periods.keys().collect::<Vec<_>>();
    sorted_periods.sort();

    // Prepare output files
    let mut tsv_output = File::create(&args.tsv_output)?;
    let mut tsv_columns: Vec<Box<dyn TsvColumns<P>>> = vec![
        PeriodColumns::new("Opened ".to_string(), |data, category| {
            data.get(category, Counter::Opened)
        }),
        PeriodColumns::new("Closed ".to_string(), |data, category| {
            data.get(category, Counter::Closed)
        }),
        PeriodColumns::new("Opened - Closed".to_string(), |data, category| {
            data.get(category, Counter::Opened) - data.get(category, Counter::Closed)
        }),
        Box::new(AccumulatedPeriodStatsFile::new().unwrap()),
    ];

    // Add headers to all files
    for output_file in &mut tsv_columns {
        output_file
            .add_headers(
                &mut tsv_output,
                &plot_data.categories,
                &plot_data.category_to_labels,
            )
            .unwrap();
    }

    // Add rows to all files
    for period in sorted_periods {
        for output_file in &mut tsv_columns {
            output_file
                .add_row(
                    &mut tsv_output,
                    period,
                    plot_data.periods.get(period).unwrap(),
                    &plot_data.categories,
                )
                .unwrap();
        }
    }

    Ok(())
}

async fn collect_data<P: Period>(args: &Args) -> PlotData<P> {
    let octocrab = octocrab::Octocrab::builder()
        .personal_token(github_api_token())
        .build()
        .unwrap();

    let mut plot_data = PlotData::new(args);

    let mut variables = Variables {
        owner: args.repo_owner(),
        name: args.repo_name(),
        page_size: args.page_size,
        after: None,
    };

    let mut page = 0;
    loop {
        page += 1;
        if page > args.pages {
            break;
        }

        let cache_path = args.cached_page_response_path(page);
        std::fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
        let response = if cache_path.exists() {
            info!("Using cached response from {}", cache_path.display());
            let file = File::open(cache_path.clone()).unwrap();

            serde_json::from_reader(&file).unwrap()
        } else {
            let response = make_github_graphql_api_request(&octocrab, &variables).await;
            info!("Caching response to {}", cache_path.display());
            atomic_write(&cache_path, &response).unwrap();
            response
        };

        let issues = &response
            .data
            .as_ref()
            .unwrap()
            .repository
            .as_ref()
            .unwrap()
            .issues;

        plot_data.analyze_issues(issues.nodes.as_ref().unwrap());

        if issues.page_info.has_next_page {
            variables.after = issues.page_info.end_cursor.clone();
        } else {
            break;
        }
    }

    plot_data
}

#[derive(Debug)]
pub struct PlotData<P: Period> {
    /// Maps a period such as "2023 May" to period data.
    periods: HashMap<P, PeriodData>,
    label_to_category: HashMap<String, IssueCategory>,
    category_to_labels: HashMap<IssueCategory, String>,
    categories: Vec<IssueCategory>,
}

impl<P: Period> PlotData<P> {
    fn new(args: &Args) -> Self {
        let mut categories = vec![];
        let mut label_to_category = HashMap::new();
        let mut category_to_labels = HashMap::new();

        for (label, category) in &args.label_category {
            if !categories.contains(category) {
                categories.push(category.clone());
            }
            label_to_category.insert(label.clone(), category.clone());
            category_to_labels
                .entry(category.clone())
                .and_modify(|labels: &mut String| labels.push_str(","))
                .or_insert_with(|| label.clone());
        }

        Self {
            periods: HashMap::new(),
            label_to_category,
            category_to_labels,
            categories,
        }
    }

    fn increment(&mut self, period: P, category: IssueCategory, counter: Counter) {
        self.periods
            .entry(period)
            .or_default()
            .increment(category, counter);
    }

    fn analyze_issues(&mut self, issues: &[Option<OpenedAndClosedIssuesRepositoryIssuesNodes>]) {
        for issue in issues.iter().flatten() {
            self.analyze_issue(issue);
        }
    }

    fn categorize_issue(
        &mut self,
        issue: &OpenedAndClosedIssuesRepositoryIssuesNodes,
    ) -> Option<IssueCategory> {
        let mut category = None;
        let labels: Vec<_> = issue
            .labels
            .iter()
            .flat_map(|labels| &labels.nodes)
            .flatten()
            .flatten()
            .collect();
        for label in &labels {
            if let Some(category_for_label) = self.label_to_category.get(&label.name) {
                category = Some(category_for_label.to_owned());
                break;
            }
        }
        if category.is_none() {
            trace!(
                "No category for issue {} with labels {:?}",
                issue.url,
                &labels
            );
            category = Some(self.label_to_category.get("*").expect("there needs to be a default category, use `--label-category '*:uncategorized'`").to_owned());
        }
        category
    }

    fn analyze_issue(&mut self, issue: &OpenedAndClosedIssuesRepositoryIssuesNodes) {
        let category = self.categorize_issue(issue).unwrap();

        self.increment(issue.created_at.into(), category.clone(), Counter::Opened);

        if let Some(closed_period) = issue.closed_at().map(|date| date.into()) {
            self.increment(closed_period, category.clone(), Counter::Closed);
        }
    }
}

type IssueCategory = String;

impl OpenedAndClosedIssuesRepositoryIssuesNodes {
    fn closed_at(&self) -> Option<DateTime> {
        if let Some(closed_at) = self.closed_at {
            Some(closed_at)
        } else if self.state == IssueState::CLOSED {
            eprintln!(
                "WARNING: issue {} has no `closed_at` but state is `CLOSED`!",
                self.url
            );
            Some(self.created_at)
        } else if let IssueState::Other(_) = self.state {
            unreachable!("Unknown issue state: {:?}", self.state);
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum PeriodEnum {
    Month,
    TwoMonths,
    Quarter,
    Year,
}
impl Display for PeriodEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PeriodEnum::Month => write!(f, "month"),
            PeriodEnum::TwoMonths => write!(f, "months"),
            PeriodEnum::Quarter => write!(f, "quarter"),
            PeriodEnum::Year => write!(f, "year"),
        }
    }
}

impl Args {
    fn repo_owner(&self) -> String {
        self.repo.split('/').next().unwrap().to_string()
    }

    fn repo_name(&self) -> String {
        self.repo.split('/').nth(1).unwrap().to_string()
    }

    /// Since we always start from page 1, we can cache
    /// queries/OpenedAndClosedIssues.graphql responses, keyed on parameters to
    /// the query.
    fn cached_page_response_path(&self, page: usize) -> PathBuf {
        let mut path = self.cached_responses_dir().clone();
        path.push(&self.repo_owner());
        path.push(&self.repo_name());
        path.push(&format!("page-size-{}", self.page_size));
        path.push(&format!("page-{}.json", page));
        path
    }

    fn cached_responses_dir(&self) -> PathBuf {
        self.cached_responses_dir.clone().unwrap_or_else(|| {
            let mut path = dirs::cache_dir().unwrap();
            path.push("enselic");
            path.push("github-repo-open-issues");
            path
        })
    }
}

async fn make_github_graphql_api_request(
    octocrab: &octocrab::Octocrab,
    variables: &Variables,
) -> graphql_client::Response<ResponseData> {
    warn!("Making GitHub GraphQL API query (affects rate limit)");
    let response: graphql_client::Response<ResponseData> = octocrab
        .graphql(
            &<OpenedAndClosedIssues as graphql_client::GraphQLQuery>::build_query(
                variables.clone(),
            ),
        )
        .await
        .unwrap();

    if let Some(errors) = response.errors {
        eprintln!("errors: {:#?}", errors);
        panic!("Got errors! See stderr above.");
    }

    response
}

fn parse_label_category(value: &str) -> anyhow::Result<(String, String)> {
    let (key, value) = value.split_once(':').unwrap();

    Ok((key.parse()?, value.parse()?))
}
