use std::{fs, io::Read};

use chrono::{DateTime, SubsecRound, Utc};
use clap::Parser;
use color_eyre::eyre::Result;
use flate2::read::GzDecoder;
use itertools::Itertools;
use k6::Record;
use minijinja::{context, Environment};
use quantogram::QuantogramBuilder;
use serde::Serialize;

use crate::k6::parse_json_result;

mod k6;

const CHUNK_SIZE: usize = 10_000;

/// Group by truncating datetime data to second unit
/// return list of vector grouped by DateTime<Utc>
fn group_to_second<'a>(data: impl Iterator<Item = &'a Record>) -> Vec<(DateTime<Utc>, Vec<f64>)> {
    data.group_by(|e| match e {
        Record::Metric { .. } => panic!(),
        Record::Point { data, .. } => data.time,
    })
    .into_iter()
    .map(|(ge0, group)| {
        (
            ge0,
            group
                .map(|e| match e {
                    Record::Metric { .. } => panic!(),
                    Record::Point { data, .. } => data.value,
                })
                .collect(),
        )
    })
    .collect()
}

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    file: String,
}

#[derive(Debug, Serialize)]
struct MeasurementPoint {
    t: DateTime<Utc>,
    vu: i64,
    avg: f64,
    p90: f64,
    p95: f64,
    p99: f64,
    rps: usize,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();

    let mut env = Environment::new();
    env.add_template("template.html", include_str!("template.html"))?;

    let compressed_content = fs::read(&args.file)?;

    let mut decoder = GzDecoder::new(compressed_content.as_slice());
    let mut json_lines = String::new();

    decoder.read_to_string(&mut json_lines)?;

    let result = parse_json_result(&json_lines);

    let mut datetimes: Vec<DateTime<Utc>> = Vec::new();
    let mut avg_response_time: Vec<f64> = Vec::new();
    let mut p90_response_time: Vec<f64> = Vec::new();
    let mut p95_response_time: Vec<f64> = Vec::new();
    let mut p99_response_time: Vec<f64> = Vec::new();
    let mut rps: Vec<usize> = Vec::new();
    let mut datetimes_vu: Vec<DateTime<Utc>> = Vec::new();
    let mut vu: Vec<i64> = Vec::new();
    let mut counter = 0;
    let mut chunk_http_req_duration = Vec::new();
    let mut chunk_http_vus = Vec::new();
    let mut last_ts = Utc::now();

    for row in result {
        let mut ts = Utc::now();
        let mut item = row.clone();

        // truncate subsecond
        match &mut item {
            Record::Metric { data: _, metric: _ } => {}
            Record::Point { data, metric: _ } => {
                data.time = data.time.trunc_subsecs(0);
                ts = data.time;
            }
        }

        match row {
            // current implementation matric data is ignored
            Record::Metric { data: _, metric: _ } => continue,
            Record::Point { data: _, metric } => {
                if metric == "http_req_duration" {
                    chunk_http_req_duration.push(item);
                } else if metric == "vus" {
                    chunk_http_vus.push(item);
                }
            }
        }

        if chunk_http_req_duration.len() >= CHUNK_SIZE && ts != last_ts {
            let groups = group_to_second(chunk_http_req_duration.iter());
            for (dt, value) in groups {
                datetimes.push(dt);
                avg_response_time.push(value.iter().sum::<f64>() / (value.len() as f64));
                rps.push(value.len());
                let mut q = QuantogramBuilder::new().with_error(0.005).build();
                q.add_unweighted_samples(value.iter());
                p90_response_time.push(q.quantile(0.90).unwrap());
                p95_response_time.push(q.quantile(0.95).unwrap());
                p99_response_time.push(q.quantile(0.99).unwrap());
            }
            chunk_http_req_duration.clear();
        }

        if chunk_http_vus.len() == CHUNK_SIZE {
            let groups = group_to_second(chunk_http_vus.iter());
            for (dt, value) in groups {
                datetimes_vu.push(dt);
                if value.len() > 1 {
                    panic!();
                }
                vu.push(value[0] as i64)
            }
            chunk_http_vus.clear()
        }

        last_ts = ts;
        counter += 1;
    }

    dbg!(counter);
    dbg!(chunk_http_req_duration.len());

    // proceess remaining data
    if !chunk_http_req_duration.is_empty() {
        let groups = group_to_second(chunk_http_req_duration.iter());
        for (dt, value) in groups {
            datetimes.push(dt);
            avg_response_time.push(value.iter().sum::<f64>() / (value.len() as f64));
            rps.push(value.len());
            let mut q = QuantogramBuilder::new().with_error(0.005).build();
            q.add_unweighted_samples(value.iter());
            p90_response_time.push(q.quantile(0.90).unwrap());
            p95_response_time.push(q.quantile(0.95).unwrap());
            p99_response_time.push(q.quantile(0.99).unwrap());
        }
        chunk_http_req_duration.clear()
    }
    if !chunk_http_vus.is_empty() {
        let groups = group_to_second(chunk_http_vus.iter());
        for (dt, value) in groups {
            datetimes_vu.push(dt);
            if value.len() > 1 {
                panic!();
            }
            vu.push(value[0] as i64)
        }
        chunk_http_vus.clear()
    }

    // build measurement vector data
    let min_count = std::cmp::min(datetimes.len(), datetimes_vu.len());
    dbg!(min_count);

    let mut data = Vec::new();

    for i in 0..min_count {
        data.push(MeasurementPoint {
            t: *datetimes.get(i).expect("failed to get datetimes"),
            vu: *vu.get(i).expect("failed to get vu element"),
            avg: *avg_response_time.get(i).expect("failed to get avg element"),
            p90: *p90_response_time.get(i).expect("failed to get p90 element"),
            p95: *p95_response_time.get(i).expect("failed to get p95 element"),
            p99: *p99_response_time.get(i).expect("failed to get p99 element"),
            rps: *rps.get(i).expect("failed to get rps element"),
        });
    }

    // handle unordered data
    let data: Vec<MeasurementPoint> = data
        .iter()
        .group_by(|e| e.t)
        .into_iter()
        .map(|(k, v)| {
            let mut vu = 0;
            let mut avg = 0.0;
            let mut p90 = 0.0;
            let mut p95 = 0.0;
            let mut p99 = 0.0;
            let mut rps = 0;
            let mut item_count = 0;
            for e in v {
                vu = e.vu;
                avg += e.avg;
                p90 += e.p90;
                p95 += e.p95;
                p99 += e.p99;
                rps += e.rps;
                item_count += 1;
            }
            MeasurementPoint {
                t: k,
                vu,
                avg: avg / (item_count as f64),
                p90: p90 / (item_count as f64),
                p95: p95 / (item_count as f64),
                p99: p99 / (item_count as f64),
                rps,
            }
        })
        .collect();

    let output = serde_json::to_string(&data)?;
    let tmpl = env.get_template("template.html")?;

    let result = tmpl.render(context!(output => output))?;

    println!("{}", result);

    Ok(())
}
