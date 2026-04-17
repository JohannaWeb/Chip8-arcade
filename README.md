# CHIP-8 Emulator

A small CHIP-8 emulator written in Rust.

<img width="835" height="492" alt="image" src="https://github.com/user-attachments/assets/93bbd57e-e90e-4304-b077-43aff99c2be2" />

<img width="835" height="492" alt="image" src="https://github.com/user-attachments/assets/2c2ac32d-c2b6-44ee-98d5-2d229408399f" />

## Requirements

Install Rust from <https://rustup.rs/>.

## Run

Launch the arcade frontend:

```powershell
cargo run --release
```

Start `RPS.ch8` with `Enter` or `Space`. Press `P` from the menu to start Pong.

RPS controls use the CHIP-8 keypad mapping:

```text
1 2 3 4
Q W E R
A S D F
Z X C V
```

In the RPS screen:

```text
Reset ROM: R
Menu:      Backspace
Quit:      Esc
```

Pong controls:

```text
Left paddle:  W / S
Right paddle: Up / Down
Reset match:  R
Menu:         Backspace
Quit:         Esc
```

Run a CHIP-8 ROM directly:

```powershell
cargo run --release -- path\to\rom.ch8
```

In ROM mode, `Esc` quits. `F1` prints the keypad layout.

## Keypad

CHIP-8 keypad:

```text
1 2 3 C
4 5 6 D
7 8 9 E
A 0 B F
```

Keyboard mapping:

```text
1 2 3 4
Q W E R
A S D F
Z X C V
```

## Test

```powershell
cargo test
```
