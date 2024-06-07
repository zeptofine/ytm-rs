use std::collections::HashMap;

use iced::widget::image::Handle;
use image::{self, GenericImageView};
use reqwest::Url;

use crate::caching::{
    readers::{CacheReader, FileData, LazyFolderBasedReader},
    IDed,
};

pub async fn get_images(
    reader: LazyFolderBasedReader,
    urls: Vec<(String, Url)>,
) -> HashMap<String, Handle> {
    // Generate paths to add to the reader.
    let filepaths = reader
        .generate_paths(urls.len())
        .map(|p| p.with_extension("png"));

    let futures = urls
        .into_iter()
        .zip(filepaths)
        .map(|((id, url), fpath)| async {
            let imgbytes = reqwest::get(url).await.unwrap().bytes().await.unwrap();
            let mut thumbnail = image::load_from_memory(&imgbytes).unwrap();
            let (w, h) = thumbnail.dimensions();

            // crop it to a square
            let smaller = h.min(w);
            let left = (w - smaller) / 2;
            let top = (h - smaller) / 2;

            thumbnail = thumbnail.crop(left, top, smaller, smaller);
            let full_path = reader.convert_path(&fpath);

            let _ = thumbnail.save(&full_path);
            (full_path, FileData::new(id, fpath))
        });

    let paths: Vec<_> = futures::future::join_all(futures)
        .await
        .into_iter()
        .collect();
    let filedatas: Vec<_> = paths.iter().map(|(_, fd)| fd).collect();
    let index_reader = reader.index_reader;

    println![
        "Extending index: {:?}",
        index_reader.extend(&filedatas, true).await
    ];

    paths
        .into_iter()
        .map(|(fp, fdata)| {
            let path = fp;
            let handle = Handle::from_path(path);

            (fdata.id().to_string(), handle)
        })
        .collect()
}
