use std::collections::HashMap;

pub use opened_and_closed_issues::*;

#[allow(clippy::upper_case_acronyms)]
pub type URI = String;
pub type DateTime = chrono::DateTime<chrono::Utc>;

#[derive(graphql_client::GraphQLQuery)]
#[graphql(
    schema_path = "schemas/github_schema.graphql",
    query_path = "queries/OpenedAndClosedIssues.graphql",
    variables_derives = "Clone, Debug",
    response_derives = "Clone, Debug, Serialize, Eq, PartialEq"
)]
pub struct OpenedAndClosedIssues;

#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub enum Counter {
    Opened,
    Closed,
}

#[derive(Debug)]
pub struct Counters(HashMap<Counter, i64>);

impl Default for Counters {
    fn default() -> Self {
        Self(HashMap::from([(Counter::Opened, 0), (Counter::Closed, 0)]))
    }
}

type IssueCategory = String;

/// Represents a period of stats. Either one week or one month depending on user
/// preference. TODO: Use &str instead of String
#[derive(Debug, Default)]
pub struct PeriodData(HashMap<IssueCategory, Counters>);

impl PeriodData {
    pub fn get(&self, category: &IssueCategory, counter: Counter) -> i64 {
        if let Some(category) = self.0.get(category) {
            if let Some(counter) = category.0.get(&counter) {
                return *counter;
            }
        }
        0
    }

    pub fn increment(&mut self, category: IssueCategory, counter: Counter) {
        *self
            .0
            .entry(category.clone())
            .or_default()
            .0
            .get_mut(&counter)
            .unwrap() += 1;
    }
}
