; pado ime's installer (Inno Setup 6): the build in windows/host/build/ime/pado, installed and registered
; as windows/install.ps1 does it. Built by windows/installer.ps1 (CI: .github/workflows/windows.yml).
;   ISCC /DAppVersion=0.3.1 windows\installer\pado.iss  →  windows\host\build\installer\pado-ime-0.3.1-setup.exe
; What it does (docs/windows.md, "Install"): Program Files\bada-windows\pado; the TIP registered for x64
; and x86 apps (regsvr32); the log folder writable by every app; then, as the user who started setup,
; windows/installer/user.ps1: pado ime added to 日本語's keyboards. Uninstall undoes all of it; settings
; and the dictionary stay.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#define Build "..\host\build\ime\pado"

[Setup]
AppId=pado-ime
AppName=pado ime
AppVerName=pado ime {#AppVersion}
AppVersion={#AppVersion}
VersionInfoVersion={#AppVersion}
AppPublisher=Mirae Research
AppPublisherURL=https://github.com/team-mirae/pado-ime
AppSupportURL=https://github.com/team-mirae/pado-ime/issues
DefaultDirName={commonpf64}\bada-windows\pado
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableReadyPage=no
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
PrivilegesRequired=admin
; Every app that types loads the TIP: they can't all be closed. A DLL in use is moved aside instead
; (PrepareToInstall), and apps load the new one when they restart.
CloseApplications=no
RestartApplications=no
LicenseFile=..\..\LICENSE
SetupIconFile=..\pado\pado.ico
UninstallDisplayIcon={app}\pado.ico
UninstallDisplayName=pado ime
OutputDir=..\host\build\installer
OutputBaseFilename=pado-ime-{#AppVersion}-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "ja"; MessagesFile: "compiler:Languages\Japanese.isl"

[Messages]
en.FinishedLabelNoIcons=pado ime is installed. Pick it with Win+Space (it's listed under 日本語), and restart apps that were running.
ja.FinishedLabelNoIcons=パドIME をインストールしました。Win+Space で選んでください（日本語の下）。起動中のアプリは再起動してください。

[Dirs]
Name: "{commonappdata}\bada-windows"; Flags: uninsneveruninstall

[Files]
Source: "{#Build}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "user.ps1"; DestDir: "{app}\tools"; Flags: ignoreversion

[UninstallDelete]
; DLLs moved aside while in use (PrepareToInstall), and whatever an app still held at uninstall.
Type: filesandordirs; Name: "{app}"

[Code]
const
  Clsid = '{27FE1FE4-13A1-49E8-8972-44F3A44EAE45}';  // windows/pado/ime.json: permanent

// A file an app has loaded can't be replaced, but it can be renamed: moved aside, the new one is copied.
procedure MoveAside(const Path: String);
var
  Dir, Name: String;
  Find: TFindRec;
begin
  Dir := ExtractFilePath(Path);
  Name := ExtractFileName(Path);
  if FindFirst(Dir + Name + '.old-*', Find) then begin
    try
      repeat
        DeleteFile(Dir + Find.Name);
      until not FindNext(Find);
    finally
      FindClose(Find);
    end;
  end;
  if FileExists(Path) and not DeleteFile(Path) then
    RenameFile(Path, Path + '.old-' + GetDateTimeString('yyyymmddhhnnsszzz', #0, #0));
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Arch: Integer;
  Dir: String;
begin
  for Arch := 0 to 1 do begin
    if Arch = 0 then Dir := ExpandConstant('{app}\x64\') else Dir := ExpandConstant('{app}\x86\');
    MoveAside(Dir + 'bada_windows.dll');
    MoveAside(Dir + 'pado_engine.dll');
  end;
  MoveAside(ExpandConstant('{app}\pado-settings.exe'));
  Result := '';
end;

function Run(const Exe, Params: String): Integer;
begin
  if not Exec(Exe, Params, '', SW_HIDE, ewWaitUntilTerminated, Result) then Result := -1;
end;

function PowerShell(const Args: String): String;
begin
  Result := '-NoProfile -ExecutionPolicy Bypass -File "' + ExpandConstant('{app}\tools\user.ps1') + '"' + Args;
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  Code: Integer;
begin
  if CurStep <> ssPostInstall then exit;
  // The log folder (%ProgramData%\bada-windows), writable by every process, Store apps included.
  Run(ExpandConstant('{sys}\icacls.exe'),
      '"' + ExpandConstant('{commonappdata}\bada-windows') + '" /grant *S-1-15-2-1:(OI)(CI)M *S-1-5-32-545:(OI)(CI)M');
  Code := Run(ExpandConstant('{sys}\regsvr32.exe'), '/s "' + ExpandConstant('{app}\x64\bada_windows.dll') + '"');
  if Code <> 0 then RaiseException('regsvr32 failed for x64 (' + IntToStr(Code) + ')');
  Code := Run(ExpandConstant('{syswow64}\regsvr32.exe'), '/s "' + ExpandConstant('{app}\x86\bada_windows.dll') + '"');
  if Code <> 0 then RaiseException('regsvr32 failed for x86 (' + IntToStr(Code) + ')');
  // The language list is per user: as the user who started setup, not the elevated account.
  if not ExecAsOriginalUser('powershell.exe', PowerShell(''), '', SW_HIDE, ewWaitUntilTerminated, Code) or (Code <> 0) then
    MsgBox('pado ime is installed, but it could not be added to your keyboards (' + IntToStr(Code) + '). ' +
           'Add it in Settings → Time & language → Language & region → 日本語 → Keyboards.', mbInformation, MB_OK);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep <> usUninstall then exit;
  // Uninstall runs elevated, as the same user when that user is an administrator.
  Run('powershell.exe', PowerShell(' -Uninstall'));
  Run(ExpandConstant('{sys}\regsvr32.exe'), '/s /u "' + ExpandConstant('{app}\x64\bada_windows.dll') + '"');
  Run(ExpandConstant('{syswow64}\regsvr32.exe'), '/s /u "' + ExpandConstant('{app}\x86\bada_windows.dll') + '"');
  RegDeleteKeyIncludingSubkeys(HKLM64, 'SOFTWARE\Microsoft\CTF\TIP\' + Clsid);
  RegDeleteKeyIncludingSubkeys(HKLM64, 'SOFTWARE\Classes\CLSID\' + Clsid);
  RegDeleteKeyIncludingSubkeys(HKLM32, 'SOFTWARE\Classes\CLSID\' + Clsid);
end;
