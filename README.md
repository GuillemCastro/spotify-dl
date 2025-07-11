# 🎵 spotify-dl

A command line utility to download songs, podcasts, playlists and albums directly from Spotify.

> [!IMPORTANT]
> A Spotify Premium account is required.

> [!CAUTION]
> Usage of this software may infringe Spotify's terms of service or your local legislation. Use it under your own risk.

## 🚀 Features

- Download individual tracks, podcasts, playlists or full albums.
- Built with Rust for speed and efficiency.
- Supports metadata tagging and organized file output.

## ⚙️ Installation

You can install it using `cargo`, `homebrew`, from source or using a pre-built binary from the releases page.

### From crates.io using `cargo`

```
cargo install spotify-dl
```

### Using homebrew (macOs)

```
brew tap guillemcastro/spotify-dl
brew install spotify-dl
```

### From source

```
cargo install --git https://github.com/GuillemCastro/spotify-dl.git
```

## 🧭 Usage

```
spotify-dl 0.9.0
A commandline utility to download music directly from Spotify

USAGE:
    spotify-dl.exe [FLAGS] [OPTIONS] <tracks>...

FLAGS:
    -F, --force      Force download even if the file already exists
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --destination <destination>    The directory where the songs will be downloaded
    -f, --format <format>              The format to download the tracks in. Default is flac. [default: flac]
    -t, --parallel <parallel>          Number of parallel downloads. Default is 5. [default: 5]

ARGS:
    <tracks>...    A list of Spotify URIs or URLs (songs, podcasts, playlists or albums)
```

Songs, playlists and albums must be passed as Spotify URIs or URLs (e.g. `spotify:track:123456789abcdefghABCDEF` for songs and `spotify:playlist:123456789abcdefghABCDEF` for playlists or `https://open.spotify.com/playlist/123456789abcdefghABCDEF?si=1234567890`).

## 📋 Examples

- Download a single track:
```bash
spotify-dl https://open.spotify.com/track/TRACK_ID
```

- Download a playlist:

```
spotify-dl -u YOUR_USER -p YOUR_PASS https://open.spotify.com/playlist/PLAYLIST_ID
```

Save as MP3 to a custom folder:
```
spotify-dl --format flac --destination ~/Music/Spotify https://open.spotify.com/album/ALBUM_ID
```

## 📄 License

spotify-dl is licensed under the MIT license. See [LICENSE](LICENSE).
