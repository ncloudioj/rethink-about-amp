import rethink_about_amp

manager = rethink_about_amp.AmpIndexManager()
manager.build_from_file("us-desktop", "data/amp-us-desktop.json")
results = manager.query("us-desktop", "am")

print(f"Found {len(results)} results")
for result in results[:3]:
    print(f"- {result.title} ({result.advertiser})")
    print(f"  Title: {result.title}")
    print(f"  Advertiser: {result.advertiser}")
    print(f"  URL: {result.url}")
    print(f"  Full keyword: {result.full_keyword}")
    print(f"  Block ID: {result.block_id}")
    print(f"  IAB Category: {result.iab_category}")
