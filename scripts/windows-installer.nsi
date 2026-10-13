Unicode True
!include "MUI2.nsh"
Name "DevKits"
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\DevKits"
InstallDirRegKey HKCU "Software\DevKits" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
!define MUI_ICON "${PAYLOAD}\devkits.ico"
!define MUI_UNICON "${PAYLOAD}\devkits.ico"
!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${PAYLOAD}\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\DevKits.exe"
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
Section "DevKits" SEC_MAIN
    SetShellVarContext current
    SetOutPath "$INSTDIR"
    File /r "${PAYLOAD}\*"
    WriteUninstaller "$INSTDIR\Uninstall.exe"
    CreateDirectory "$SMPROGRAMS\DevKits"
    CreateShortcut "$SMPROGRAMS\DevKits\DevKits.lnk" "$INSTDIR\DevKits.exe" "" "$INSTDIR\devkits.ico"
    CreateShortcut "$SMPROGRAMS\DevKits\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
    WriteRegStr HKCU "Software\DevKits" "InstallDir" "$INSTDIR"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\DevKits" "DisplayName" "DevKits"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\DevKits" "DisplayVersion" "${VERSION}"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\DevKits" "UninstallString" '"$INSTDIR\Uninstall.exe"'
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\DevKits" "DisplayIcon" "$INSTDIR\devkits.ico"
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\DevKits" "NoModify" 1
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\DevKits" "NoRepair" 1
SectionEnd
Section "Uninstall"
    SetShellVarContext current
    Delete "$SMPROGRAMS\DevKits\DevKits.lnk"
    Delete "$SMPROGRAMS\DevKits\Uninstall.lnk"
    RMDir "$SMPROGRAMS\DevKits"
    Delete "$INSTDIR\DevKits.exe"
    Delete "$INSTDIR\devkits.ico"
    Delete "$INSTDIR\README.md"
    Delete "$INSTDIR\LICENSE"
    Delete "$INSTDIR\licenses\noto-sans-mono\OFL.txt"
    Delete "$INSTDIR\licenses\noto-sans-mono\README.txt"
    RMDir "$INSTDIR\licenses\noto-sans-mono"
    RMDir "$INSTDIR\licenses"
    Delete "$INSTDIR\Uninstall.exe"
    RMDir "$INSTDIR"
    DeleteRegKey HKCU "Software\DevKits"
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\DevKits"
SectionEnd
