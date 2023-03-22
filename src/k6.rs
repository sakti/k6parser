use std::cmp::Ordering;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// DataMetric detail
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct DataMetric {
    #[serde(rename = "type")]
    pub typ: String,
    pub contains: String,
    // pub tainted: serde_json::Value,
    pub thresholds: serde_json::Value,
    pub submetrics: serde_json::Value,
}

/// DataPoint detail
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataPoint {
    pub time: DateTime<Utc>,
    pub value: f64,
    pub tags: serde_json::Value,
}

impl Eq for DataPoint {}

impl PartialEq for DataPoint {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

/// Record type wrapper for k6 output result
#[derive(Serialize, Deserialize, Debug, Clone, Eq)]
#[serde(tag = "type")]
pub enum Record {
    Metric { data: DataMetric, metric: String },
    Point { data: DataPoint, metric: String },
}

impl PartialEq for Record {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Metric {
                    data: _,
                    metric: l_metric,
                },
                Self::Metric {
                    data: _,
                    metric: r_metric,
                },
            ) => l_metric == r_metric,
            (
                Self::Point {
                    data: l_data,
                    metric: l_metric,
                },
                Self::Point {
                    data: r_data,
                    metric: r_metric,
                },
            ) => l_data.time == r_data.time && l_metric == r_metric,
            (Record::Metric { data: _, metric: _ }, Record::Point { data: _, metric: _ }) => false,
            (Record::Point { data: _, metric: _ }, Record::Metric { data: _, metric: _ }) => false,
        }
    }
}

impl PartialOrd for Record {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Record::Metric { data: _, metric: _ }, Record::Metric { data: _, metric: _ }) => {
                Some(Ordering::Equal)
            }
            (Record::Metric { data: _, metric: _ }, Record::Point { data: _, metric: _ }) => {
                Some(Ordering::Less)
            }
            (Record::Point { data: _, metric: _ }, Record::Metric { data: _, metric: _ }) => {
                Some(Ordering::Less)
            }
            (
                Record::Point {
                    data: l_data,
                    metric: _,
                },
                Record::Point {
                    data: r_data,
                    metric: _,
                },
            ) => l_data.time.partial_cmp(&r_data.time),
        }
    }
}

impl Ord for Record {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Record::Metric { data: _, metric: _ }, Record::Metric { data: _, metric: _ }) => {
                Ordering::Equal
            }
            (Record::Metric { data: _, metric: _ }, Record::Point { data: _, metric: _ }) => {
                Ordering::Less
            }
            (Record::Point { data: _, metric: _ }, Record::Metric { data: _, metric: _ }) => {
                Ordering::Less
            }
            (
                Record::Point {
                    data: l_data,
                    metric: _,
                },
                Record::Point {
                    data: r_data,
                    metric: _,
                },
            ) => l_data.time.cmp(&r_data.time),
        }
    }
}

/// Parse Json line result from k6 output.
pub fn parse_json_result(input: &str) -> Vec<Record> {
    let result: Vec<Record> = input
        .lines()
        // add rayon to enable this
        // .par_bridge()
        .filter(|v| !v.trim().is_empty())
        .map(|v| serde_json::from_str::<Record>(v).unwrap())
        .collect();

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_result() {
        let payload = r#"
{"type":"Metric","data":{"type":"gauge","contains":"default","thresholds":[],"submetrics":null},"metric":"vus"}
{"type":"Point","data":{"time":"2017-05-09T14:34:45.625742514+02:00","value":5,"tags":null},"metric":"vus"}
{"type":"Metric","data":{"type":"trend","contains":"time","thresholds":["avg<1000"],"submetrics":null},"metric":"http_req_duration"}
{"type":"Point","data":{"time":"2017-05-09T14:34:45.239531499+02:00","value":459.865729,"tags":{"group":"::my group::json","method":"GET","status":"200","url":"https://httpbin.org/get"}},"metric":"http_req_duration"}
		"#;
        assert_eq!(parse_json_result(payload).len(), 4);
    }
}

