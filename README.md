Explore the alternative ways to handle AMP suggestions in Merino.

### What's in the repo

#### Data
Various datasets are included in the `data` directory.
- `amp-us-desktop.json`: the entire suggestion assets for US/Desktop.
- `amp-us-mobile.json`: the entire suggestion assets for US/Mobile.
- Breakdown parts for `amp-us-desktop` (raw file size: 2.0MB)
  - `advertisers.json` (16KB): the `advertiser` field for all suggestions.
      - Lots of repeatitive values.
  - `title.json` (44KB): the `title` field for all suggestion.
  - `click_urls.json` (184KB): the `click_url` field.
      - All URLs share the same host prefix.
  - `impression_urls.json` (132KB): the `click_url` field.
      - All URLs share the same host prefix.
  - `urls.json` (184KB): the `url` field.
  - `kw.json` (1.1MB): the `keyword` field.
      - Accounts for 55% of the file, highly redundant.
  - `fk.json` (408KB): the `full_keyword` field.
  - `iab.json` (20KB): the `iab_category` field.
      - Share the identical value ("22 - Shopping"). Can be hardcoded.
  - `icons.json` (24KB): the `iab_category` field.
      - Lots of repeative values.

### Ideas

#### Collapse Keywords & Partials

Instead of storing the entire keywords & partials, we can collapse those that sharing the same prefix to save space and speed up the query time.

```
# Original keywords & partials
fo
foo
foob
fooba
foobar

# After collapsing, the following carrise the same information.
# It reads the keyword `foobar` and its partials start from the 1st character (0-based).
foobar, 1
```

Note that if we store keywords & partials this way, we can't use a hash index for the keywords & partials, a tree-based (e.g. btree) index is more suitable, especially those ones that support range queries.

For example, if a user types "fo" in the URL bar, Merino will issue a point lookup for "fo" against this index. "foobar" would be returned, then it'll the the partial beginning index, which is "1" in this case, i.e. "fo" is indeed a valid prefix for "foobar", hence it's a hit.

With this handling, the original keywords & partials are reduced from 61K items to 11K items.

There is also an example of performing prefix queries against a `BTreeMap` in Rust.

#### Remove Duplicate Full Keywords
Full keywords are currently encoded via Run-Length Encoding. However, there are still lots of duplicates with the collapsed keywords & partials generated above. We could only store those full keywords which are not the same as the collapsed keywords. For example, we will store the full keyword "amazon" for "amazin" but not for the full keyword "amazon fresh" as the collapsed keyword is also "amazon fresh".

While Run-Length Encoding saves space, it is hard to reference a full keyword given a 0-based index as it needs to scan the full keyword list and sum up the "runs" until the summation is equal or greater than the index. We can consider switching to an RLE variant – Run-End Encoding. Specifically, instead of store the "runs", it store the ending index for the value. For example, for an RLE: `[("foo", 3), ("bar", 4), ("baz", 2)]`, its REE is `[("foo", 2), ("bar", 6), ("baz", "8")]`. Then binary search can be used to reference a value by its index.

#### Group Suggestions by Advertisers
To better levarage Run-Length-Encoding (or Run-End-Encoding), we can group suggestions by advertiser as suggestions in the same group usually share a lot of fields such as "advertiser", "title", and "icon".

#### Templatize URL Fields
For "click_url", "impression_url", and "url", they tend to share the same prefix for the same advertiser, it's possible to templatize those URLs via Dictionary Encoding to reduce redundancy. For example, if an advertiser has the two URLs: "https://www.foo.com/product?param01=bar" and "https://www.foo.com/product?param01=baz". We can templatize them as "{PREFIX-KEY-01}?param01=bar" and "{PREFIX-KEY-01}?param01=baz", respectively, where "{PREFIX-KEY-01}" points to "https://www.foo.com/product" in a prefix dictionary.

# Building Python package

```bash
> pipx install maturin
> maturin develop --features python
```

# Running Python library
```
> python test_call.py
Found 1 results

Result 1:
  Title: Amazon.com - Official Site
  Advertiser: Amazon
  URL: https://www.amazon.com/?tag=admarketus-20&ref=pd_sl_924ab4435c5a5c23aa2804307ee0669ab36f88caee841ce51d1f2ecb&mfadid=adm
  Full keyword: Amazon
  Block ID: 59
  IAB Category: 22 - Shopping
```
