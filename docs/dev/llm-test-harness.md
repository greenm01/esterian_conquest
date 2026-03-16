# LLM Automated Test Harness for Esterian Conquest

This document outlines the architecture for creating an automated, LLM-driven testing harness for the original Esterian Conquest (EC) v1.5 DOS executable.

## The Goal
To allow an LLM (like Claude, GPT-4, or Gemini) to "play" the original game autonomously. The LLM needs to see the game's menus, understand the state, and issue keystrokes so we can observe how the game engine modifies its underlying `.DAT` files. This black-box testing is critical for reverse-engineering the game's exact mechanics for the Rust reimplementation.

## The Architecture: The Enigma BBS Bot
Originally, we attempted to bypass the BBS entirely and connect directly to DOSBox's nullmodem serial port. However, due to complex DTR/DSR serial handshaking requirements and modern SDL/Wayland headless window crashing bugs, the most robust path is to simply act as a real Telnet user logging into your Enigma BBS.

The testing pipeline looks like this:
1. **The Python Bot:** A Python script connects to the Enigma BBS via Telnet.
2. **Login Automation:** The bot sends the test username (`<YOUR_TEST_USER>`) and password (`<YOUR_TEST_PASS>`), navigates the BBS menu, and launches the Esterian Conquest door.
3. **The Raw ANSI Capture:** Once the game launches, the bot starts recording the raw Telnet byte stream to a `.ans` file, preserving all original DOS box-drawing and color codes exactly as they were rendered by the game.
4. **The Headless Terminal (`pyte`):** The bot feeds that raw byte stream into `pyte`, a Python-based in-memory VT100 emulator. 
5. **The LLM Interface:** `pyte` strips out all the confusing cursor jumping and color codes, exposing a perfectly formatted 80x25 plain-text array of the screen. The bot sends this clean text array to the LLM.
6. **The Interaction Loop:** The LLM reads the plain-text menu, decides on an action, and the Python bot sends that keystroke (e.g., `b"1\r"`) down the Telnet pipe, triggering the next screen render.

## Why this works so well for LLMs:
- **Spatial Awareness:** The LLM can easily read tabular data, read gold/ship counts at the top of the screen, and see what menu options are available because the text is laid out exactly as it appears visually.
- **Predictability:** The game uses single-key inputs (like hitting `A` for Attack). The LLM is very good at outputting specific, constrained commands based on a visual text prompt.

## Setup Instructions

**Note on Security:** It is recommended to create a dedicated testing account in your Enigma BBS with restricted permissions instead of using your primary Sysop account credentials in your automated scripts.

1. **Install Dependencies:**
   Since Arch Linux uses externally managed Python environments, you should run the bot inside a virtual environment.
   ```bash
   python3 -m venv /tmp/venv
   source /tmp/venv/bin/activate
   pip install pyte
   ```

2. **Run the Bot:**
   Use the provided tool in the repository to launch the bot. By default, it connects to localhost and uses dummy credentials. Pass your specific test credentials via arguments.
   ```bash
   python tools/bbs/llm_bbs_bot.py --host 127.0.0.1 --port 8888 --user testuser --password testpass
   ```

The script will handle the BBS login, capture the screen, and dump a human-readable text block of exactly what the LLM will see!