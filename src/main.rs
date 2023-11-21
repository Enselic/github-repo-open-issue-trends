use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::{collections::HashMap, path::PathBuf};

use serde::Serialize;
use tracing::*;

mod models;
use models::*;

mod tsv_output_file;
use tsv_output_file::*;

mod period;
use period::*;

#[derive(Debug, Clone, clap::ValueEnum)]
enum PeriodEnum {
    Month,
    TwoMonths,
}

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

    /// How many issues were opened in each month, .tsv output path.
    #[arg(long, default_value = "opened-issues.tsv")]
    opened_issues_output: PathBuf,

    /// How many issues were closed in each month, .tsv output path.
    #[arg(long, default_value = "closed-issues.tsv")]
    closed_issues_output: PathBuf,

    /// How many issues are open in total at the end of each month, .tsv output path.
    #[arg(long, default_value = "open-issues.tsv")]
    open_issues_output: PathBuf,

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
    }
}

async fn run_main<P: Period>(args: &Args) -> anyhow::Result<()> {
    // Collect data
    let plot_data = collect_data(&args).await;
    let mut sorted_periods = plot_data.periods.keys().collect::<Vec<_>>();
    sorted_periods.sort();

    // Prepare output files
    let mut tsv_output_files: Vec<Box<dyn TsvOutputFile<P>>> = vec![
        Box::new(PeriodStatsFile::new(&args.opened_issues_output, Counter::Opened).unwrap()),
        Box::new(PeriodStatsFile::new(&args.closed_issues_output, Counter::Closed).unwrap()),
        Box::new(AccumulatedPeriodStatsFile::new(&args.open_issues_output).unwrap()),
    ];

    // Add headers to all files
    for output_file in &mut tsv_output_files {
        output_file.add_headers(&plot_data.categories).unwrap();
    }

    // Add rows to all files
    for period in sorted_periods {
        for output_file in &mut tsv_output_files {
            output_file
                .add_row(
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
    categories: Vec<IssueCategory>,
}

impl<P: Period> PlotData<P> {
    fn new(args: &Args) -> Self {
        let mut categories = vec![];
        let mut label_to_category = HashMap::new();

        for (label, category) in &args.label_category {
            if !categories.contains(category) {
                categories.push(category.clone());
            }
            label_to_category.insert(label.clone(), category.clone());
        }

        Self {
            periods: HashMap::new(),
            label_to_category,
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

fn github_api_token() -> String {
    let output = std::process::Command::new("git")
        .arg("config")
        .arg("--get")
        .arg("github.oauth-token")
        .output()
        .unwrap();

    if output.status.success() {
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    } else {
        panic!("No GitHub token configured. To configure, run: git config github.oauth-token <your-token>")
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

fn atomic_write(dest_path: &Path, data: &impl Serialize) -> std::io::Result<()> {
    let mut tmp_path = dest_path.to_owned();
    tmp_path.set_extension("tmp");

    let tmp_file = File::create(&tmp_path)?;
    serde_json::to_writer(&tmp_file, &data)?;
    tmp_file.sync_all()?;

    std::fs::rename(tmp_path, dest_path)
}

fn parse_label_category(value: &str) -> anyhow::Result<(String, String)> {
    let (key, value) = value.split_once(':').unwrap();

    Ok((key.parse()?, value.parse()?))
}
