# ytm-rs

Ok so apparently a LOT of people are making music players in Rust like right now. 
This was my introduction to the world of Rust and it has been very fun so i decided to unprivate this git repo.



## Installation

Make sure you have Cargo installed and your toolchains are up to date.
Currently, you also need Python for the backend.

First, create a virtual environment and install the dependencies.

```bash
python -m venv .venv
source .venv/bin/activate # or .venv/Scripts/activate.bat on Windows
# Install the dependencies
python -m pip install -e .
# Launch the application
cargo run
```

### Prebuilds will be provided once this project is in a good state.


### TODO:
- [x] Tree-based queue system
- [x] queue system UI
- [x] Drag 'n dropping items
- [x] Music player UI like playback, progress, etc.
- [x] Requesting songs from ytm
- [ ] Starting the next song after the current song is done.
- [~] Searching for music
  - [ ] Loading songs
  - [x] Loading tabs (playlists, artists, etc.)
  - [x] Loading search results (Songs finished, need to make a Tab view)
- [x] Downloading songs from youtube
- [x] Thumbnails for songs, albums and playlists.
- [ ] Saving playlists/albums as a SongOperation
- [x] Actually playing music.
- [ ] Dropping unused sounds from the cache