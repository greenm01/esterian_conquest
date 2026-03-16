# Local BBS Door Testing Setup

To validate our Rust data-layer against the original DOS `ECMAINT.EXE` and `ECBBS.EXE` binaries in a realistic environment, we maintain a test harness using [Enigma BBS](https://github.com/NuSkooler/enigma-bbs).

This setup allows developers to `telnet` into a local BBS, launch the DOS game via DOSBox-X, make turns, and observe the resulting `*.DAT` files before/after the Rust backend processes them.

## 1. Prerequisites
You need Node.js (for Enigma BBS) and `dosbox-x` (for the game binaries). `dosbox-x` is highly recommended over standard `dosbox` for BBS doors because it has much better serial port, socket, and headless compatibility.

```bash
# Example for NixOS, adjust for your OS or install via Flatpak
nix-env -iA nixpkgs.dosbox-x
```

## 2. Enigma BBS Setup
Clone Enigma BBS somewhere outside the repo (e.g., your home directory):

```bash
cd ~
git clone https://github.com/NuSkooler/enigma-bbs.git
cd enigma-bbs
npm install
./oputil.js config new
```
Follow the wizard, set a BBS name, and accept the default ports (Telnet usually binds to `8888`).

## 3. Configuring the Esterian Conquest Door
We provide a wrapper script at `tools/bbs/run_ec_dos.sh` that mounts the game directory and injects the dropfile.

In your `enigma-bbs` directory, create or edit the file `config/menus/doors.hjson` to add Esterian Conquest:

```hjson
{
  esterianConquest: {
    desc: Esterian Conquest
    module: extern_sys
    config: {
      # Make sure this points to the absolute path of the repository script!
      cmd: /path/to/esterian_conquest/tools/bbs/run_ec_dos.sh
      args: [
        "{dropFilePath}/DOOR.SYS",
        "{node}"
      ]
      io: socket
    }
  }
}
```

Then, hook this door into your main BBS menu so you can launch it. Edit `config/menus/main.hjson` (or wherever your `xtrn` hotkey is configured) to point to the `esterianConquest` key defined above.

## 4. Testing
Start the BBS:
```bash
cd ~/enigma-bbs
node main.js
```

In another terminal, connect:
```bash
telnet localhost 8888
```

Login, navigate to the door menu, and launch Esterian Conquest. DOSBox-X will spawn to run `ECBBS.EXE`.
