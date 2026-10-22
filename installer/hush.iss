; Hush installer (Inno Setup 6). Per-user install, no admin rights needed.
;
; Build (scripts\release.ps1 does this for you):
;   ISCC.exe installer\hush.iss                     version read from Cargo.toml
;   ISCC.exe /DAppVersion=1.2.3 installer\hush.iss  explicit version
;   ISCC.exe /DExeDir=..\target\debug ...           package another build
; Output: installer\Output\Hush-Setup-<version>.exe

#define AppName "Hush"
#define AppExe "hush-voice.exe"

#ifndef ExeDir
  #define ExeDir "..\target\release"
#endif

; Version comes from the [package] version line in Cargo.toml.
#ifndef AppVersion
  ; Cargo.toml reads as INI: [package] version = "x.y.z".
  #define AppVersion StringChange(Trim(ReadIni(AddBackslash(SourcePath) + "..\Cargo.toml", "package", "version", "")), '"', '')
#endif
#if AppVersion == ""
  #error Could not read the version from Cargo.toml
#endif

[Setup]
AppId={{8F1C6B2E-5A0D-4F2B-9C3E-6A7D1B2C3D4E}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=ijon007
AppPublisherURL=https://github.com/ijon007/hush-voice
AppSupportURL=https://github.com/ijon007/hush-voice/issues
AppUpdatesURL=https://github.com/ijon007/hush-voice/releases
VersionInfoVersion={#AppVersion}
DefaultDirName={localappdata}\Programs\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
DisableDirPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
SetupIconFile=..\assets\logo\icon-dark.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
OutputDir=Output
OutputBaseFilename=Hush-Setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=force

[Files]
Source: "{#ExeDir}\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{userprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"

[Run]
Filename: "{app}\{#AppExe}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{sys}\taskkill.exe"; Parameters: "/IM {#AppExe} /F"; Flags: runhidden; RunOnceId: "StopHush"

[Registry]
; "Launch at login" lives in the per-user Run key; remove it on uninstall.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueName: "{#AppName}"; Flags: uninsdeletevalue dontcreatekey
