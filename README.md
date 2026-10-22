# Hush

## Releasing

Releases are made by hand from a Windows machine with the build tools above,
[Inno Setup 6](https://jrsoftware.org/isinfo.php) (`winget install JRSoftware.InnoSetup`)
and the [GitHub CLI](https://cli.github.com/) (`winget install GitHub.cli`, then `gh auth login` once).

```powershell
.\scripts\release.ps1                 # release the version already in Cargo.toml
.\scripts\release.ps1 -Version 0.2.0  # bump Cargo.toml, commit "Release v0.2.0", then release
.\scripts\release.ps1 -Draft          # same, but the GitHub release starts as a draft
```

The script needs a clean working tree. It builds `cargo build --release` inside the
MSVC environment, packages `installer\Output\Hush-Setup-X.Y.Z.exe` with
`installer\hush.iss`, tags `vX.Y.Z`, pushes the commit and tag, and creates the
GitHub release with the installer attached and generated notes. Installed copies of
Hush see the new release on their next launch ("Update available").
