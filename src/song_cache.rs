use std::{
    collections::{HashMap, HashSet},
    io::BufRead,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use async_std::prelude::*;

use crate::{
    settings::{project_directory, SongKey},
    song::{to_hash_map, Song},
};

fn _songs_path() -> PathBuf {
    let mut path = project_directory();
    path.push("songs.ndjson");
    path
}

#[derive(Debug, Clone)]
pub struct LineSongPair(pub String, pub Song);

pub type SongCacheMap = HashMap<SongKey, Arc<Mutex<Song>>>;

#[derive(Debug, Clone)]
pub struct SongCache {
    map: SongCacheMap,
    lock: Arc<Mutex<()>>,
    pub filepath: PathBuf,
}

impl Default for SongCache {
    fn default() -> Self {
        Self {
            map: Default::default(),
            lock: Default::default(),
            filepath: _songs_path(),
        }
    }
}

impl SongCache {
    /// Filters out the songs in saved_songs that have only one refcount,
    /// meaning they are no longer being used by anything other than the map
    pub fn find_unused_songs(&self) -> impl Iterator<Item = String> + '_ {
        self.map.iter().filter_map(|(key, s)| {
            let count = Arc::strong_count(s);
            match count == 1 {
                true => Some(key.clone()),
                false => None,
            }
        })
    }

    pub fn drop_from_cache(&mut self, keys: impl IntoIterator<Item = String>) {
        keys.into_iter().for_each(|key| {
            self.map.remove(&key);
        });
    }

    pub fn fetch(&mut self, songids: &HashSet<SongKey>) -> SongCacheMap {
        let cs_keys: HashSet<SongKey> = self.map.keys().cloned().collect();

        match cs_keys.is_empty() {
            true => self.cache_from_ndjson(songids).collect(),
            false => {
                let already_cached = songids.intersection(&cs_keys);
                let not_cached: HashSet<_> = songids.difference(&cs_keys).cloned().collect();

                // chain caches from ndjson if not_cached is not empty
                let get_cache = (!not_cached.is_empty())
                    .then(|| {
                        self.cache_from_ndjson(&not_cached)
                            .collect::<Vec<(String, Arc<Mutex<Song>>)>>()
                    })
                    .into_iter()
                    .flatten();
                already_cached
                    .into_iter()
                    .map(|k| (k.clone(), self.new_arc(k)))
                    .chain(get_cache)
                    .collect()
            }
        }
    }

    /// Creates an iterator from reading the ndjson file and creating songs.
    /// The iterator yields a tuple of the original line and the created song
    pub fn read_songs_from_ndjson(
        pth: impl AsRef<Path>,
    ) -> Result<impl Iterator<Item = LineSongPair>, std::io::Error> {
        println!["Reading from ndjson"];
        let pth = pth.as_ref();
        std::fs::File::open(pth).map(|file| {
            std::io::BufReader::new(file)
                .lines()
                // Filter to lines that successfully read
                .map_while(Result::ok)
                // Filter to lines that are valid songs
                .filter_map(|l| {
                    serde_json::from_str::<Song>(&l)
                        .ok()
                        .map(|s| LineSongPair(l, s))
                })
        })
    }

    fn cache_from_ndjson(
        &mut self,
        songids: &HashSet<SongKey>,
    ) -> impl Iterator<Item = (String, Arc<Mutex<Song>>)> + '_ {
        (!songids.is_empty())
            .then(|| {
                let songs: Vec<_> = {
                    let _lock = self.lock.lock().unwrap();
                    Self::read_songs_from_ndjson(&self.filepath)
                        .unwrap()
                        .filter_map(move |LineSongPair(_, song)| {
                            let id = song.id.clone();

                            let contains = songids.contains(&id);
                            contains.then(|| (id, Arc::new(Mutex::new(song))))
                        })
                        .collect()
                };
                self.map.extend(songs.clone());
                songs.into_iter().map(|(str, _)| {
                    let arc = self.new_arc(&str);
                    (str, arc)
                })
            })
            .into_iter()
            .flatten()
    }

    fn new_arc(&self, songid: &SongKey) -> Arc<Mutex<Song>> {
        Arc::clone(&self.map[songid])
    }

    /// Extends the cache with new songs. This is different from extend() in that this uses the map's filelock and precaches the new songs.
    pub async fn extend_file(
        &mut self,
        songs: impl Iterator<Item = Song>,
        overwrite: bool,
    ) -> Result<(), async_std::io::Error> {
        let mut songs: HashMap<String, Song> = to_hash_map(songs);

        let songs_path = &self.filepath;
        let tempfile = songs_path.with_extension("ndjson.tmp");
        {
            // Create the temporary file to write the new songlist to
            let output_file = async_std::fs::File::create(&tempfile).await?;
            let mut out = async_std::io::BufWriter::new(output_file);

            // Read the original songlist
            {
                let valid_songs: Vec<Vec<u8>> = {
                    let _lock = self.lock.lock().unwrap();
                    if let Ok(songlist) = SongCache::read_songs_from_ndjson(songs_path) {
                        // Filter through the lines, find existing keys and skip broken lines
                        filter_file_songs(songlist, overwrite, &mut songs).collect()
                    } else {
                        vec![vec![]]
                    }
                };
                for bytes in valid_songs {
                    out.write(&bytes).await?;
                }

                out.flush().await?;
            }

            // Add remaining keys to the file
            for (id, song) in songs {
                let mut json = serde_json::to_string(&song).unwrap();
                json.push('\n');
                out.write(json.as_bytes()).await?;

                // Add song to cache
                self.map.insert(id, Arc::new(Mutex::new(song)));
            }
            out.flush().await?;
        }
        // Replace songs.ndjson with songs.ndjson.tmp
        {
            let _lock = self.lock.lock().unwrap();
            std::fs::rename(&tempfile, songs_path)?;
        }

        Ok(())
    }

    pub async fn extend(
        pth: impl AsRef<Path>,
        songs: impl Iterator<Item = Song>,
        overwrite: bool,
    ) -> Result<(), async_std::io::Error> {
        let mut songs: HashMap<String, Song> = to_hash_map(songs);

        let songs_path = pth.as_ref();
        let tempfile = songs_path.with_extension("ndjson.tmp");

        {
            // Create the temporary file to write the new songlist to
            let output_file = async_std::fs::File::create(&tempfile).await?;
            let mut out = async_std::io::BufWriter::new(output_file);
            {
                // Read the original songlist
                if let Ok(songlist) = SongCache::read_songs_from_ndjson(songs_path) {
                    // Filter through the lines, find existing keys and skip broken lines
                    let valid_songs = filter_file_songs(songlist, overwrite, &mut songs);

                    for bytes in valid_songs {
                        out.write(&bytes).await?;
                    }

                    out.flush().await?;
                }
            }

            // Add remaining keys to the file
            for (_id, song) in songs {
                let mut json = serde_json::to_string(&song).unwrap();
                json.push('\n');
                out.write(json.as_bytes()).await?;
            }
            out.flush().await?;
        }
        // Replace songs.ndjson with songs.ndjson.tmp
        async_std::fs::rename(&tempfile, &songs_path).await?;

        Ok(())
    }
}

#[derive(Default, Debug, Clone)]
pub struct CacheInterface {
    cache: SongCacheMap,
    keys: HashSet<SongKey>,
}

impl CacheInterface {
    pub fn get<'a>(
        &'a self,
        songids: &'a HashSet<SongKey>,
    ) -> impl Iterator<Item = (String, Arc<Mutex<Song>>)> + '_ {
        let existing = songids.intersection(&self.keys).cloned();
        existing.map(|k| (k.clone(), Arc::clone(&self.cache[&k])))
    }

    pub fn extend(&mut self, songlist: SongCacheMap) {
        let new_keys: HashSet<SongKey> = songlist.keys().cloned().collect();
        self.keys = self.keys.union(&new_keys).cloned().collect();
        self.cache.extend(songlist)
    }

    pub fn pop(&mut self, songlist: impl IntoIterator<Item = String>) -> SongCacheMap {
        let new_cache: SongCacheMap = songlist
            .into_iter()
            .filter_map(|k| self.cache.remove_entry(&k))
            .collect();
        self.keys = new_cache.keys().cloned().collect();
        new_cache
    }

    pub fn replace(&mut self, cache: SongCacheMap) {
        self.keys = cache.keys().cloned().collect();
        self.cache = cache;
    }

    /// Returns the number of elements in the cache1

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn get_keys(&self) -> &HashSet<String> {
        &self.keys
    }
}

fn filter_file_songs<'a>(
    songlist: impl Iterator<Item = LineSongPair> + 'a,
    overwrite: bool,
    songs: &'a mut HashMap<String, Song>,
) -> impl Iterator<Item = Vec<u8>> + 'a {
    songlist.filter_map(move |LineSongPair(mut line, song)| {
        line.push('\n');
        match (overwrite, songs.contains_key(&song.id)) {
            (true, true) => None,
            (true, false) => Some(line.as_bytes().to_vec()),
            (false, true) => {
                songs.remove(&song.id);
                Some(line.as_bytes().to_vec())
            }
            (false, false) => Some(line.as_bytes().to_vec()),
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::{settings::SongKey, song::Song};
    use once_cell::sync::Lazy;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::{collections::HashSet, sync::Arc};
    use tempfile::TempDir;

    static TESTING_LOCK: Lazy<Arc<Mutex<()>>> = Lazy::new(|| Arc::new(Mutex::new(())));

    static TEST_FOLDER: Lazy<TempDir> = Lazy::new(|| tempfile::tempdir().unwrap());

    fn random_file() -> PathBuf {
        TEST_FOLDER
            .path()
            .join(format!("{:x}", rand::random::<u64>()))
    }

    use super::SongCache;

    #[test]
    fn fetching() {
        let songs = vec![Song::basic(), Song::basic(), Song::basic()];
        let missing_songs = vec![Song::basic(), Song::basic(), Song::basic()];
        let real_keys: HashSet<SongKey> = songs.iter().map(|s| s.id.clone()).collect();
        let keys: HashSet<SongKey> = real_keys
            .iter()
            .cloned()
            .chain(missing_songs.clone().iter().map(|s| s.id.clone()))
            .collect();

        let tmpfile = random_file();
        let mut sc = SongCache {
            lock: TESTING_LOCK.clone(),
            filepath: tmpfile,
            ..Default::default()
        };

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let r = runtime.block_on(sc.extend_file(songs.clone().into_iter(), false));
        println!["{:?}", r];
        assert![r.is_ok()];

        // Sending the same songs to check the overwriting mechanism
        let r = runtime.block_on(sc.extend_file(songs.into_iter(), false));
        println!["{:?}", r];
        assert![r.is_ok()];
        let songs = sc.fetch(&keys);
        println!["{:?}", sc];
        println!["{:?}", songs];
        assert_eq![songs.len(), 3];
    }

    #[test]
    fn checking_unused() {
        let songs = vec![Song::basic(), Song::basic()];
        let first_key = songs[0].id.clone();
        let keys: HashSet<SongKey> = songs.clone().iter().map(|s| s.id.clone()).collect();

        let tmpfile = random_file();
        let mut sc = SongCache {
            lock: TESTING_LOCK.clone(),
            filepath: tmpfile,
            ..Default::default()
        };

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let extension_result = { runtime.block_on(sc.extend_file(songs.into_iter(), false)) };
        println!["{extension_result:?}"];
        assert![extension_result.is_ok()];

        sc.fetch(&keys);
        let mut unused: Vec<_> = sc.find_unused_songs().collect();
        println!["{sc:?}"];
        println!["Unused songs before: {:?}", unused];
        assert_eq![unused.len(), 2];
        {
            let guards = sc.fetch(&[first_key.clone()].into_iter().collect());
            assert![guards.len() > 0];
            println!["Captured guards: {:?}", guards];
            let song = guards[&first_key].lock();
            println!["Captured song: {:?}", song];

            unused = sc.find_unused_songs().collect();
            println!["Unused songs during: {:?}", unused];
            assert_eq![unused.len(), 1];
        }
        unused = sc.find_unused_songs().collect();
        println!["Unused songs after: {:?}", unused];
        assert_eq![unused.len(), 2];
    }

    #[test]
    fn dropping() {
        let songs = vec![Song::basic(), Song::basic()];
        let first_key = songs[0].id.clone(); // the drop target

        let tmpfile = random_file();
        let mut sc = SongCache {
            lock: TESTING_LOCK.clone(),
            filepath: tmpfile,
            ..Default::default()
        };

        let runtime = tokio::runtime::Runtime::new().unwrap();
        {
            let r = runtime.block_on(sc.extend_file(songs.into_iter(), false));
            println!["{r:?}"];
            assert![r.is_ok()];
        }
        let mut unused: Vec<_> = sc.find_unused_songs().collect();
        assert_eq![unused.len(), 2];
        sc.drop_from_cache([first_key]);
        unused = sc.find_unused_songs().collect();
        assert_eq![unused.len(), 1];

        sc.drop_from_cache(unused);
        unused = sc.find_unused_songs().collect();
        assert_eq![unused.len(), 0];
    }
}
