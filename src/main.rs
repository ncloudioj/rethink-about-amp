use std::{collections::BTreeMap, fs::File, io::BufReader};

use itertools::Itertools;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Amp {
    pub keywords: Vec<String>,
    pub title: String,
    pub url: String,
    pub score: Option<f64>,
    #[serde(default)]
    pub full_keywords: Vec<(String, usize)>,
    pub advertiser: String,
    #[serde(rename = "id")]
    pub block_id: i32,
    pub iab_category: String,
    pub click_url: String,
    pub impression_url: String,
    #[serde(rename = "icon")]
    pub icon_id: String,
}

fn main() {
    let mut m: BTreeMap<_, _> = BTreeMap::new();
    m.insert("abc", 1);
    m.insert("abd", 2);
    m.insert("aba", 0);
    m.insert("ab8", -1);
    m.insert("abç", 3);
    m.insert("cde", 3);

    // Search for a given prefix against the tree.
    // `\u{10FFFF}` is the largest unicode code point.
    for (k, v) in m.range("ab"..concat!("ab", '\u{10FFFF}')) {
        println!("{k}: {v}");
    }

    merge_kw();
}

fn merge_kw() {
    let file = File::open("data/amp-us-desktop.json").unwrap();
    let reader = BufReader::new(file);
    let suggestions: Vec<Amp> = serde_json::from_reader(reader).unwrap();

    let mut total: usize = 0;
    let mut merged: usize = 0;

    // Collapse the AMP keywords by merging the consectutive commonly-prefixed partials.
    for suggestion in suggestions.iter() {
        total += suggestion.keywords.len();
        let m = suggestion
            .keywords
            .iter()
            .coalesce(|prev, curr| {
                if curr.starts_with(prev) && prev.chars().count() + 1 == curr.chars().count() {
                    Ok(curr)
                } else {
                    Err((prev, curr))
                }
            })
            .collect_vec();
        merged += m.len()
    }

    println!("{total}; {merged}");

    let kw = vec![
        "am",
        "ama",
        "amaz",
        "amazi",
        "amazin",
        "amazo",
        "amazon",
        "amazon ",
        "amazon f",
        "amazon fr",
        "amazon fre",
        "amazon fres",
        "amazon fresh",
        "amazon l",
        "amazon lo",
        "amazon log",
        "amazon logi",
        "amazon login",
        "amazon p",
        "amazon pr",
        "amazon pri",
        "amazon prim",
        "amazon prime",
        "amazon prime ",
        "amazon prime l",
        "amazon prime lo",
        "amazon prime log",
        "amazon prime logi",
        "amazon prime login",
        "amazon prime v",
        "amazon prime vi",
        "amazon prime vid",
        "amazon prime vide",
        "amazon prime video",
        "amazon prin",
        "amazon pro",
        "amazon ri",
        "amazon u",
        "amazon us",
        "amazon usa",
        "amon",
        "amon ",
        "amos",
        "amos ",
        "amso",
        "bmaz",
        "bmazi",
        "bmazin",
    ];

    // Collapse the keywords and calculate the prefix beginning index.
    for (k, i) in
        kw.into_iter()
            .map(|w| (w, w.chars().count()))
            .coalesce(|(prev, lp), (curr, lc)| {
                if curr.starts_with(prev) && prev.chars().count() + 1 == lc {
                    Ok((curr, lp))
                } else {
                    Err(((prev, lp), (curr, lc)))
                }
            })
    {
        println!("{k}, {i}");
    }
}
