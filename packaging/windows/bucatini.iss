; Inno Setup script for Bucatini (Windows x64).
; Build with: iscc /DMyAppVersion=X.Y.Z /DSourceDir=<staged> /DOutputDir=<out> bucatini.iss
; The NDI 6 Runtime is NOT bundled — users install it once separately.

#ifndef MyAppVersion
  #define MyAppVersion "0.0.0"
#endif
#ifndef SourceDir
  #define SourceDir "."
#endif
#ifndef OutputDir
  #define OutputDir "dist"
#endif

#define MyAppName "Bucatini"
#define MyAppPublisher "naporin0624"
#define MyAppExeName "bucatini-gui.exe"
#define MyAppIco SourceDir + "\bucatini.ico"

[Setup]
; A stable AppId keeps upgrades/uninstall consistent across versions.
AppId={{7B0CA710-0F1E-4A2B-9C3D-000000000001}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDir}
OutputBaseFilename=Bucatini-{#MyAppVersion}-windows-x64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
SetupIconFile={#MyAppIco}
UninstallDisplayIcon={app}\bucatini.ico

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\bucatini.exe";        DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\bucatini-gui.exe";    DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\bucatini.ico";        DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\README.md";           DestDir: "{app}"; Flags: ignoreversion isreadme
Source: "{#SourceDir}\README.ja.md";        DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\THIRD-PARTY-NOTICES"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}";        Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\bucatini.ico"
Name: "{group}\Uninstall {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}";  Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\bucatini.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#MyAppName}}"; Flags: nowait postinstall skipifsilent
