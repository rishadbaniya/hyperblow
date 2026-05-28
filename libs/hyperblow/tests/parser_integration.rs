use hyperblow::parser::{magnet_uri_parser::MagnetURIMeta, torrent_parser::FileMeta};

struct ParserFixture;

impl ParserFixture {
    fn sample_single_file_torrent() -> Vec<u8> {
        b"d8:announce30:udp://tracker.example.com:69694:infod6:lengthi12345e4:name10:sample.bin12:piece lengthi16384e6:pieces20:abcdefghijklmnopqrstee"
            .to_vec()
    }
}

#[test]
fn parses_single_file_torrent_metadata() {
    let meta = FileMeta::fromRawTorrentFile(ParserFixture::sample_single_file_torrent()).expect("sample torrent should parse");

    assert_eq!(meta.announce, "udp://tracker.example.com:6969");
    assert_eq!(meta.info.name.as_deref(), Some("sample.bin"));
    assert_eq!(meta.total_length(), 12_345);
    assert_eq!(meta.piece_count(), 1);
    assert_eq!(meta.getPiecesHash().expect("pieces should be valid")[0], *b"abcdefghijklmnopqrst");
    assert_eq!(meta.generateInfoHash().len(), 20);
}

#[test]
fn rejects_piece_hashes_that_are_not_twenty_byte_chunks() {
    let invalid = b"d8:announce30:udp://tracker.example.com:69694:infod6:lengthi1e4:name1:x12:piece lengthi1e6:pieces3:abcee".to_vec();
    let meta = FileMeta::fromRawTorrentFile(invalid).expect("bencode itself is valid");

    assert!(meta.getPiecesHash().is_err());
}

#[test]
fn parses_magnet_metadata_with_trackers() {
    let magnet = MagnetURIMeta::fromMagnetURI(
        "magnet:?xt=urn:btih:08ada5a7a6183aae1e09d831df6748d566095a10&dn=Sintel&xl=123&tr=udp://tracker.example.com:6969",
    )
    .expect("magnet should parse");

    assert_eq!(magnet.xt.as_deref(), Some("urn:btih:08ada5a7a6183aae1e09d831df6748d566095a10"));
    assert_eq!(magnet.dn.as_deref(), Some("Sintel"));
    assert_eq!(magnet.xl, Some(123));
    assert_eq!(magnet.tr.unwrap(), vec!["udp://tracker.example.com:6969"]);
}
