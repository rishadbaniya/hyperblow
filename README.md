# Hyperblow - A torrent client that throws real left blow

**Currently this project is in a complete rewrite**

[![dependency status](https://deps.rs/repo/github/rishadbaniya/hyperblow/status.svg)](https://deps.rs/repo/github/rishadbaniya/hyperblow)


It's gonna take time, good things do take time

✅ Denotes work is fully done
☑️ Means work is partially done
☐ Means the work is far from done

## Features checklist :
- ✅ Accepts torrent file as input
- ✅ Accepts magnet uri as input
- ☑️ Support for partial download, that is checking the items we want to download
- ✅ Support for UDP Trackers
- ☐ Support for HTTP Trackers
- ☐ Has rare piece first algorithm
- ☐ Implements Choking and Unchoking Algorithm

Supported BEP's:

- ✅ [BEP15](http://www.bittorrent.org/beps/bep_0015.html) : UDP Tracker Protocol (Implements partially, except scrape req and res)
- ✅ [BEP12](http://bittorrent.org/beps/bep_0012.html) : MultiTracker Metadat Extension
- ✅ [BEP20](https://www.bittorrent.org/beps/bep_0020.html) : Peer ID Convention

TODO : 
- ✅ Implement the ".torrent" file parser
- ✅ Implement the MagnetURI verifier and parser
- ✅ Handle redundancy of both the tracker URL's in "announce" and "announce-list" field, used BEP12
- ☐ Make use of Crossbeam crate's Concurrency Primitives
- ☐ Add both Unit and Integration testing for the parsing library 
- ☐ Re architect the entire CLI application's system design

