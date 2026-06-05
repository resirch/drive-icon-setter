# drive-icon-setter

Rust desktop app for setting a Windows drive icon to a custom `.ico` file. PNG files are accepted and converted to `.ico` when needed.

It uses the registry method described in the ElevenForum guide:

- Current user: `HKCU\Software\Classes\Applications\Explorer.exe\Drives\<LETTER>\DefaultIcon`
- All users: `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\DriveIcons\<LETTER>\DefaultIcon`

## Usage

Launch the GUI:

```powershell
cargo run
```

Or use the CLI:

```powershell
cargo run -- F C:\Icons\Backup.ico
cargo run -- F C:\Icons\Backup.png
cargo run -- F C:\Windows\Backup.ico --scope machine
cargo run -- F --remove
```

`--scope user` is the default and affects only the current account. `--scope machine` affects all users and must be run from an elevated terminal.

Windows expects an `.ico` file for this method. When you choose a PNG, the app writes a sibling `.ico` file next to it and points the registry at that file. Close and reopen File Explorer after changing or removing an icon.
