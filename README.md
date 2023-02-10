# spotify-dl

A command line utility to download songs and playlists directly from Spotify's servers.

You need a Spotify Premium account.

## Dependencies

spotify-dl depends on libflac

### Debian-based distros

```
sudo apt install libflac-dev libasound2-dev
```
### Red Hat-based distros

```
sudo dnf install flac-devel alsa-lib-devel
```

### MacOSX

```
brew install flac
```

## Usage

```
spotify-dl 0.1.0
A commandline utility to download music directly from Spotify

USAGE:
    spotify-dl [FLAGS] [OPTIONS] --username <username> [tracks]...

FLAGS:
    -h, --help       Prints help information
    -o, --ordered    Download songs in the order they are in the playlist, prfixing the filename with the track number
    -V, --version    Prints version information

OPTIONS:
    -d, --destination <destination>    The directory where the songs will be downloaded [default: .]
    -p, --password <password>          Your Spotify password
    -u, --username <username>          Your Spotify username

ARGS:
    <tracks>...    A list of Spotify URIs (songs, podcasts or playlists)
```

Songs and playlists must be passed as Spotify URIs or URLs (e.g. `spotify:track:123456789abcdefghABCDEF` for songs and `spotify:playlist:123456789abcdefghABCDEF` for playlists or `https://open.spotify.com/playlist/123456789abcdefghABCDEF?si=1234567890`).

## Disclaimer

The usage of this software may infringe Spotify's ToS and/or your local legislation. Use it under your own risk.

## License

spotify-dl is licensed under the MIT license. See [LICENSE](LICENSE).
