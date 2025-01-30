!Include MUI2.nsh
Var _1
Var _2
Var _10
Var _15
Var _16
Var _21
Var _28
Var _32
Name "@projectName@ (@projectVersion@)"
OutFile "@outFileName@"
!define MUI_ICON "@installerIconPath@"
!include WinVer.nsh
VIProductVersion 0.0.0.0
VIAddVersionKey "ProductVersion" @projectVersion@
Unicode true
RequestExecutionLevel Highest
InstallDir "$PROGRAMFILES64\@projectName@"
InstallDirRegKey HKLM "Software\@projectName@" "Install_Dir"
!insertmacro MUI_LANGUAGE "English"
LangString AlreadyRunning ${LANG_ENGLISH} "is running. It needs to be fully shut down before running the installer!"
LangString TooOld ${LANG_ENGLISH} "This version of Windows is not supported. Windows 8.1 or above required."
!define MUI_PAGE_CUSTOMFUNCTION_PRE PreDirectory
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!addplugindir "EnVar"
Section "" _sec14
  SectionIn RO
  SetOutPath "$INSTDIR"
  AllowSkipFiles off
  WriteRegStr HKLM "Software\@projectName@" "Install_Dir" "$INSTDIR"
  CreateDirectory "$APPDATA\@projectName@\Secrets-1.0"
  CreateDirectory "$APPDATA\@projectName@\Logs"
  CreateDirectory "$APPDATA\@projectName@\Logs\pub"
  StrCpy $_15 "0"
  StrCpy $_16 ""
  _lbl17:
  IntCmp "$_15" "30" 0 _lbl23 0
  StrCpy $_21 ""
  Goto _lbl20
  _lbl23:
  StrCmpS "$_16" "" 0 _lbl30
  StrCpy $_28 "1"
  Goto _lbl31
  _lbl30:
  StrCpy $_28 ""
  _lbl31:
  StrCpy $_21 "$_28"
  StrCmpS "$_21" "" _lbl20 0
  IntOp $_32 "$_15" "+" "1"
  DetailPrint "Checking if @projectName@ is not running ($_32\30)..."
  StrCpy $_16 "1"
  ClearErrors
  Delete "$APPDATA\@projectName@\@lockfileName@"
  IfErrors 0 _lbl33
  StrCpy $_16 ""
  Goto _lbl36
  _lbl33:
  StrCmpS "$_16" "" 0 _lbl37
  _lbl36:
  Sleep "1000"
  _lbl37:
  IntOp $_15 "$_15" "+" "1"
  Goto _lbl17
  _lbl20:
  StrCmpS "$_16" "" 0 _lbl45
  Abort "@projectName@ $(AlreadyRunning)"
  _lbl45:
  IfFileExists "$INSTDIR" 0 _lbl52
  DetailPrint "Removing previously installed version"
  RMDir /r "$INSTDIR"
  _lbl52:
  File /r "contents\*"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\@projectName@" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\@projectName@" "Publisher" "IOHK"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\@projectName@" "ProductVersion" "@projectVersion@"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\@projectName@" "VersionMajor" "0"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\@projectName@" "VersionMinor" "0"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\@projectName@" "DisplayName" "@projectName@"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\@projectName@" "DisplayVersion" "@projectVersion@"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\@projectName@" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\@projectName@" "QuietUninstallString" "$\"$INSTDIR\uninstall.exe$\" \S"
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\@projectName@" "NoModify" "1"
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\@projectName@" "NoRepair" "1"
  EnVar::SetHKLM
  EnVar::AddValue "path" "$INSTDIR"
SectionEnd
Section "Start Menu Shortcuts" _sec62
  CreateDirectory "$SMPROGRAMS\@projectName@"
  CreateShortcut "$SMPROGRAMS\@projectName@\Uninstall @projectName@.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\uninstall.exe" "0" "" "" ""
SectionEnd
Function .onInit
  ${IfNot} ${AtLeastWin8.1}
    MessageBox MB_OK "$(TooOld)"
    Quit
  ${EndIf}
  ReadRegStr $_1 HKLM "Software\@projectName@" "Install_Dir"
  StrCpy $_2 "$_1"
FunctionEnd
Function PreDirectory
  StrLen $_10 "$_2"
  StrCmpS "$_10" "0" _lbl5 0
  Abort ""
  _lbl5:
FunctionEnd
