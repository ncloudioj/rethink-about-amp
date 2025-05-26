//! Analyze the `keywords` and `full_keywords` fields for redundancy inspection.

use std::iter::repeat_n;

use rethink_about_amp::*;

fn main() {
    // Load data once
    println!("Loading AMP data...");
    let amps = load_amp_data("data/amp-us-desktop.json").unwrap();
    println!("Loaded {} AMP suggestions", amps.len());

    let combined: Vec<(String, String, bool)> = amps
        .iter()
        .flat_map(|suggestion| {
            // Restore the pointwise full keywords sequence via the RLE encoded. `full_keywords`.
            let full_keywords =
                suggestion
                    .full_keywords
                    .iter()
                    .flat_map(|(full_keyword, repeat_for)| {
                        repeat_n(full_keyword.as_str(), *repeat_for)
                    });

            // Zip up the keywords with the full_keywords.
            let keywords_ext: Vec<(_, _)> = suggestion
                .keywords
                .iter()
                .map(String::as_str)
                .zip(full_keywords)
                .collect();

            let mut out = Vec::<(String, String, bool)>::new();
            let mut i = 0;
            while i < keywords_ext.len() {
                let (curr, curr_fk) = keywords_ext[i];

                let curr_len = curr.chars().count();
                let mut j = i + 1;
                let mut n_collapsed = 0;

                // extend the run as long as each next is curr + exactly one char
                while j < keywords_ext.len() {
                    let (nxt, _) = keywords_ext[j];
                    if nxt.starts_with(curr) && nxt.chars().count() == curr_len + n_collapsed + 1 {
                        n_collapsed += 1;
                        j += 1;
                    } else {
                        break;
                    }
                }

                assert_eq!(j, i + n_collapsed + 1);
                if j > i + 1 {
                    // we saw a run [i .. j), so collapse to keywords_ext[j-1]
                    let (kw, fk) = keywords_ext[j - 1];
                    out.push((kw.to_string(), fk.to_string(), fk == kw));
                    i = j;
                } else {
                    out.push((curr.to_string(), curr_fk.to_string(), curr == curr_fk));
                    i += 1;
                }
            }
            out
        })
        .collect();

    println!("Total entries: {}", combined.len());
    println!(
        "Total entries with the identical full keywords: {}",
        combined.iter().filter(|(_, _, p)| *p).count()
    );
    for (kw, fk, same) in combined.iter().take(20) {
        println!("{kw}; {fk}; {same}");
    }
}
