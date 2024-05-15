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
- [ ] Music player UI like playback, progress, etc.
- [x] Requesting songs from ytm
- [ ] Searching for music
  - [ ] Loading songs
  - [x] Loading tabs (playlists, artists, etc.)
  - [ ] Loading search results
- [ ] Downloading songs from youtube
- [ ] Thumbnails for songs, albums and playlists.
- [ ] Saving playlists/albums as a SongOperation
- [ ] Actually playing music. (Don't look at me like that. I have launching individual songs working using kittyaudio. I will switch to rodio/kira once they get proper song tracking support.)