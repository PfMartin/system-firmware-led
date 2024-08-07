# system-firmware-led

## Description

Repository for firmware of led IoT devices.

## Development setup

### Project setup

The project has been set up with the following steps:

- Install espup with cargo `cargo install espup`
- Install necessary toolchains `espup install` -> Creates `$HOME/export-esp.sh`
- Source the `$HOME/export-esp.sh` script or create an alias and execute the alias

```bash
# .zshrc

alias get_esprs='. $HOME/export-esp.sh'
```

- Make sure python3.12-venv is installed `sudo apt install python3.12-venv`
- Install ldproxy with cargo `cargo install ldproxy`
- Create the project using a template
  - Make sure cargo-generate is installed `cargo install cargo-generate`
  - Generate the template with `cargo generate esp-res/esp-idf-template cargo` (uses std approach)
    - Give the project a name
    - Select `esp32c3`
    - 'Configure advanced template options?' -> false

### Flash the micro controller

- Make sure you use a USB Cable that supports data transmission
- Make sure you plugin the micro controller
- Make sure `export-esp.sh` is exported -> Use one of the following
  - `. ~/export-esp.sh`
  - `get_esprs`
- Run `cargo run` -> Automatically selects the correct USB-Port
