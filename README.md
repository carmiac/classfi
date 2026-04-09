# Classfi - A Simple Classical Music Player

Heavily inspired by [lowfi](https://github.com/talwat/lowfi), but for classical music from [Classical California](https://www.classicalcalifornia.org/) streams.

## TODO

### 0.1.0 Release

- [ ] Write some documentation
- [ ] CI testing
- [ ] CI release binary

### Future

- [ ] cache station URLs to disk
- [ ] Stream info (file type, bitrate, etc)
- [ ] better player state reporting/tracking
  - [ ] test via flakey connection tools
  - [ ] look at ignored messages from mpv
    - Getting URL -> finding stream
    - Got URL -> found stream
    - StartFile -> Connecting
    - FileLoaded -> Connected
    - PlaybackRestart -> Playing
- [ ] add media key handling
- [ ] one line UI
- [ ] Create alternate versions with different sources
  - https://docs.rs/clap/latest/clap/_cookbook/multicall_busybox/index.html

## References

[jellyfin-tui](https://github.com/austinwilcox/jellyfin-tui/tree/main/src/player/mpv.rs)

[mpv.io](https://mpv.io/manual/master)

## License

Copyright (c) Adam Milner <carmiac@gmail.com>

This project is licensed under the GPLv3 ([LICENSE] or <https://www.gnu.org/licenses/gpl-3.0.html>)

[LICENSE]: ./LICENSE
