mod automation;
mod commands;
mod cookie;
mod dash;
mod downloader;
mod hls;
mod logger;
mod playlist;
mod progress;
mod utils;

#[doc(hidden)]
pub use commands::Args;
pub use reqwest;

// use anyhow::Result;
// use downloader::{MAX_RETRIES, MAX_THREADS, SKIP_MERGE, Decrypter};
// use reqwest::{Client, Url};
// use std::path::PathBuf;

// /// A downloader for DASH and HLS playlists.
// pub struct Downloader {
//     pub input: String,
//     pub base_url: Option<Url>,
//     pub directory: Option<PathBuf>,
//     pub output: Option<PathBuf>,
//     pub subs_codec: String,
//     pub interactive: bool,
//     pub interactive_raw: bool,
//     pub select_streams: String,
//     pub keys: Decrypter,
//     pub no_decrypt: bool,
//     pub retries: u8,
//     pub threads: u8,
//     pub client: Client,
// }

// impl Downloader {
//     pub fn new(client: Client, input: String) -> Self {
//         Self {
//             input,
//             base_url: None,
//             directory: None,
//             output: None,
//             subs_codec: "copy".to_string(),
//             interactive: false,
//             interactive_raw: false,
//             select_streams: "v=best:s=en".to_string(),
//             keys: Decrypter::None,
//             no_decrypt: false,
//             no_merge: false,
//             retries: 5,
//             threads: 5,
//             client,
//         }
//     }

//     pub fn base_url(mut self, base_url: impl Into<Url>) -> Self {
//         self.base_url = Some(base_url.into());
//         self
//     }

//     pub fn directory(mut self, directory: impl Into<PathBuf>) -> Self {
//         self.directory = Some(directory.into());
//         self
//     }

//     pub fn output(mut self, output: impl Into<PathBuf>) -> Self {
//         self.output = Some(output.into());
//         self
//     }

//     pub fn subs_codec(mut self, subs_codec: impl Into<String>) -> Self {
//         self.subs_codec = subs_codec.into();
//         self
//     }

//     pub fn interactive(mut self, raw: bool) -> Self {
//         self.interactive = interactive;
//         self
//     }

//     /// Sets the stream selection filters.
//     pub fn select_streams(mut self, select_streams: impl Into<String>) -> Self {
//         self.select_streams = select_streams.into();
//         self
//     }

//     /// Consumes the builder and returns a configured [`Downloader`].
//     pub fn build(self) -> Downloader {
//         Downloader {
//             input: self.input,
//             base_url: self.base_url,
//             directory: self.directory,
//             output: self.output,
//             subs_codec: self.subs_codec,
//             interactive: self.interactive,
//             interactive_raw: self.interactive_raw,
//             select_streams: self.select_streams,
//             keys: self.keys,
//             no_decrypt: self.no_decrypt,
//             no_merge: self.no_merge,
//             retries: self.retries,
//             threads: self.threads,
//             client: self.client,
//         }
//     }
// }

// // pub async fn execute(client: &Client) -> Result<()> {
// //     MAX_RETRIES.store(self.retries, Ordering::SeqCst);
// //     MAX_THREADS.store(self.threads, Ordering::SeqCst);
// //     SKIP_MERGE.store(self.no_merge, Ordering::SeqCst);

// //     let client = self.client()?;

// //     let prompter = Prompter {
// //         interactive: self.interactive,
// //         interactive_raw: self.interactive_raw,
// //     };

// //     let meta = downloader::fetch_playlist(
// //         self.base_url.clone(),
// //         &client,
// //         &self.input,
// //         &prompter,
// //         &self.query,
// //     )
// //     .await?;

// //     if self.list_streams {
// //         downloader::list_all_streams(&meta)?;
// //     } else if self.parse {
// //         let playlist =
// //             downloader::parse_all_streams(self.base_url.clone(), &client, &meta, &self.query)
// //                 .await?;
// //         serde_json::to_writer(std::io::stdout(), &playlist)?;
// //     } else {
// //         let streams = downloader::parse_selected_streams(
// //             self.base_url.clone(),
// //             &client,
// //             &meta,
// //             &prompter,
// //             &self.query,
// //             SelectOptions::parse(&self.select_streams),
// //         )
// //         .await?;

// //         downloader::download(
// //             self.base_url,
// //             client,
// //             self.keys,
// //             self.directory,
// //             self.no_decrypt,
// //             self.output,
// //             self.query,
// //             streams,
// //             self.subs_codec,
// //         )
// //         .await?;
// //     }

// //     Ok(())
// // }
