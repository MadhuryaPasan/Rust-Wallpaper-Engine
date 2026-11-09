; Version and paths come from scripts/package.ps1, which reads Cargo.toml.
#ifndef AppVersion
  #error AppVersion is required. Run scripts/package.ps1.
#endif
#ifndef SourceDir
  #error SourceDir is required. Run scripts/package.ps1.
#endif
#ifndef OutputDir
  #error OutputDir is required. Run scripts/package.ps1.
#endif
#ifndef NumericVersion
  #error NumericVersion is required. Run scripts/package.ps1.
#endif

[Setup]
; Keep this ID stable across releases so upgrades replace the existing install.
AppId={{D4D6507F-3A66-4DC3-99D0-5942ECBB937A}
AppName=Serein
AppVersion={#AppVersion}
VersionInfoVersion={#NumericVersion}
VersionInfoProductVersion={#AppVersion}
AppPublisher=Serein contributors
DefaultDirName={localappdata}\Programs\Serein
DefaultGroupName=Serein
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
OutputDir={#OutputDir}
OutputBaseFilename=Serein-{#AppVersion}-windows-x64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\serein-studio.exe
CloseApplications=yes
CloseApplicationsFilter=serein-studio.exe,serein-player.exe
RestartApplications=no
SetupLogging=yes

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: unchecked

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\Serein"; Filename: "{app}\serein-studio.exe"; WorkingDir: "{app}"
Name: "{group}\Uninstall Serein"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Serein"; Filename: "{app}\serein-studio.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\serein-studio.exe"; Description: "Open Serein"; Flags: nowait postinstall skipifsilent

; No UninstallDelete entries: imported images, profiles and drafts belong to users.
