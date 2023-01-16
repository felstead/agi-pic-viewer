# AGI PIC Viewer

AGI PIC viewer is a UI application for viewing PIC resources in the old-school 1980s Sierra On-line games like King's Quest, Space Quest, Leisure Suit Larry, etc.  PIC resources are the backgrounds for the game's screens.  They are interesting in that they are not stored in a typical raster format, rather they are drawn at runtime with a series of vector drawing commands.  This app handles the v2 version of the resource files, present in KQ1, KQ2, KQ4 and KQ4 AGI, SQ1, SQ2, PQ1, etc.  It doesn't handle SCI files.

![Screenshot of AGI PIC Viewer showing King's Quest 2](https://github.com/felstead/agi-pic-viewer/blob/master/misc/screenshot.png?raw=true)

The app is written in Rust and uses the excellent [egui](https://egui.rs) UI framework.  I am relatively new to the Rust language, so I wouldn't suggest using this code as a reference for much of anything.  It is also more or less completely unoptimized, but parses and renders all the screens from a game in a few seconds on my machine, so is certainly usable.

## Installation

Make sure you have Rust installed, then use `cargo` to build the project.  I'd suggest building `--release` if you want it to be faster.

```bash
cargo build --release
```

## Usage

To view the content for an AGI game, you must have the game installed on your machine.  Please note, that in spite of their age, many of the classic Sierra On-line games are still for sale on sites like gog.com, for example:

* [Kings Quest 1, 2 & 3](https://www.gog.com/en/game/kings_quest_1_2_3)
* [Space Quest 1, 2 & 3](https://www.gog.com/en/game/space_quest_1_2_3)

Fun fact: KQ1 was the first PC game I pirated as a kid back in the day, so to make up for that I bought the games off GoG to do this project.

After you build, run the executable, passing in the path to the game whose resources you want to see as the first argument, e.g.

On Windows:
```cmd
agi-pic-render.exe "C:\Program Files (x86)\GOG Galaxy\Games\Kings Quest 3\"
```

On OSX or Linux:
```bash
agi-pic-render /path/to/some/game/
```

If you can't or don't want to shell out the money for these games, there are several of these older AGI games that are no longer sold and can be considered Abandonware, such as Manhunter, Mixed-up Mother Goose or The Black Cauldron, and they can be found out on the web without too much trouble.

## Contributing

Pull requests are welcome. For major changes, please open an issue first
to discuss what you would like to change.

## License

[MIT](https://choosealicense.com/licenses/mit/)