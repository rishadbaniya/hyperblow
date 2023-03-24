use clap::Parser;

#[derive(Debug, Parser, Default)]
#[clap(author = "Rishad Baniya", version)]
pub struct Arguments {
    #[arg(short('f'))]
    /// Path to the torrent file you wish to download
    pub torrent_file: Option<String>,

    /// URI of the torrent file you wish to download
    #[arg(short('m'))]
    pub magnet_uri: Option<String>,
}

impl Arguments {
    /// Checks if the torrent_file argument provided or not, doesn't validate by checking
    /// if the file exists, or is a valid bencode encoded torrent file or not
    pub fn is_file_argument_provided(&self) -> bool {
        self.torrent_file != None
    }

    /// Checks if the magnet_uri argument provided or not, doesn't validate by checking
    /// if the magnet uri is valid or not
    pub fn is_magnet_uri_provided(&self) -> bool {
        self.magnet_uri != None
    }

    /// Checks if both arguments are provided
    pub fn is_both_argument_provided(&self) -> bool {
        self.is_file_argument_provided() && self.is_magnet_uri_provided()
    }

    /// Checks if none of the arguments are provided
    pub fn none_arguments_provided(&self) -> bool {
        !(self.is_file_argument_provided() || self.is_magnet_uri_provided())
    }

    /// Checks if arguments are provided or not, if none of the arguments are provided
    /// and both arguments are provided then the program shall panic for now
    ///
    /// TODO: Run in idle mode when no arguments are provided, similar to VIM where it simply loads
    /// up the buffer region to type, load a some sort of similar region to load torrent files
    pub fn check(&self) {
        if self.is_both_argument_provided() {
            // TODO: Support providing both Magnet URI and Torrent File as input
            todo!("Can't provided both Magnet URI and Torrent File as source as of right now!")
        } else if self.none_arguments_provided() {
            // TODO: Just like VIM opens a buffer region when we don't specify a file, open an idle
            // region similarly
            todo!("None of the arguments were provided")
        }
    }
}
