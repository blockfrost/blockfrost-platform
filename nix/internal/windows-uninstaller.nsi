!Include MUI2.nsh
Unicode true
!insertmacro MUI_LANGUAGE "English"
Name "@projectName@ @projectVersion@ Uninstaller"
OutFile "tempinstaller.exe"
SetCompress off
RequestExecutionLevel Highest
!addplugindir "EnVar"
Section "" _sec1
  SectionIn RO
  WriteUninstaller "c:\uninstall.exe"
SectionEnd
Section "Uninstall" _sec2
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\@projectName@"
  DeleteRegKey HKLM "Software\@projectName@"
  RMDir /r /rebootok "$INSTDIR"
  Delete "$SMPROGRAMS\@projectName@\*.*"
  Delete "$DESKTOP\@projectName@.lnk"
  EnVar::SetHKLM
  EnVar::DeleteValue "path" "$INSTDIR"
SectionEnd
