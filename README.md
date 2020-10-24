# spotify-dl

A command line utility to download songs and playlists directly from Spotify's servers.

You need a Spotify Premium account.

## Dependencies

spotify-dl depends on libflac

### Debian-based distros

```
sudo apt install libflac-dev
```

## Usage

```
spotify-dl 0.1.0
A commandline utility to download music directly from Spotify

USAGE:
    spotify-dl [OPTIONS] --username <username> [tracks]...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --destination <destination>    The directory where the songs will be downloaded [default: .]
    -u, --username <username>          Your Spotify username

ARGS:
    <tracks>...    A list of Spotify URIs (songs, podcasts or playlists)
```

Songs and playlists must be passed as Spotify URIs (e.g. `spotify:track:123456789abcdefghABCDEF` for songs and `spotify:playlist:123456789abcdefghABCDEF` for playlists).

## License

spotify-dl is licensed under the MIT license. See [LICENSE](LICENSE).