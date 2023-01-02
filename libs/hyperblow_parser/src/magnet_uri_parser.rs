#![allow(non_snake_case, dead_code)]

// TODO: Add unit and integration test
use magnet_url::Magnet;

pub enum MagnetURIMetaError {
    URIError(String),
}

// TODO: Implement MagnetURIMeta
/// DataStructure that maps all the data withing a Magnet URI into something rust program can use.
struct MagnetURIMeta {
    /// **(Required)** Exact Topic : Info Hash of the torrent
    xt: Option<Vec<u8>>,

    /// **(Optional)** Display name : The filename to display to the user
    dn: Option<String>,

    /// **(Optional)** Exact Length : The size of the file in bytes
    xl: Option<u64>,

    /// **(Optional)** Address Tracker : The url of the tracker
    tr: Option<String>,

    /// **(Optional)** Web Seed : They payload data served over HTTP(S)
    ws: Option<String>,
}

impl MagnetURIMeta {
    //fn fromMagnetURI(uri: &String) -> Result<MagnetURIMeta, MagnetURIMetaError> {
    //    return match Magnet::new(uri) {
    //        Ok(d) => {
    //            // Convert Magnet into MagnetURIMeta
    //            //Ok(MagnetURIMeta { dn: d.dn })
    //        }
    //        Err(e) => Err(MagnetURIMetaError::URIError("Error in your given URI".to_string())),
    //    };
    //}

    //fn parseMagnetURI(uri: &String) -> Result<MagnetURIMeta, MagnetURIMetaError> {
    //    Err(MagnetURIMetaError("Error".to_string()))
    //}

    /// Just Checks if the Magnet URI is valid or not
    fn checkIfMagnetURIIsValid(uri: &String) -> bool {
        return match Magnet::new(uri) {
            Ok(_) => true,
            Err(_) => false,
        };
    }
}
